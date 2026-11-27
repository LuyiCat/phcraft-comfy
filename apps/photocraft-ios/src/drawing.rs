//! iOS's drawing policy and the pencil's pointer.
//!
//! iOS alone decides who may draw, fill and select, from the device idiom UIKit reports and
//! never from the screen's size, which an unknown model or a multitasking mode could misjudge:
//! the iPad's is the Apple Pencil alone (`DrawPolicy::PencilOnly`) and the iPhone's is nobody
//! (`DrawPolicy::None`). Android keeps the shared default and has no counterpart to anything here.
//!
//! On the iPad a finger is never drawing input, so where only the pencil draws the pencil also
//! owns the pointer ([`PencilPointer`]). egui-winit emulates the pointer from the first touch
//! that starts and keeps it until that touch ends, and the canvas reads its strokes from that
//! pointer. So while a finger held the pointer the pencil could not draw until it lifted and
//! landed again — the resting hand is all it takes. [`PencilPointer`] ends a finger's canvas press
//! when the pencil lands and writes the pencil's pointer events itself for as long as egui-winit
//! writes none, so the stroke begins where the pencil lands and ends where it lifts.

// Only the iOS shell runs the pencil's pointer; elsewhere its tests are what compile it.
#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

use egui::{Event, PointerButton, Pos2, TouchId, TouchPhase};
use photocraft_ui_egui::draw_policy::DrawPolicy;

/// The policy of the device the app runs on: the iPad's pencil alone, nobody on the iPhone.
#[cfg(target_os = "ios")]
pub fn device_policy() -> DrawPolicy {
    use objc2_ui_kit::{UIDevice, UIUserInterfaceIdiom};
    let pad = objc2::MainThreadMarker::new().is_some_and(|mtm| UIDevice::currentDevice(mtm).userInterfaceIdiom() == UIUserInterfaceIdiom::Pad);
    if pad { DrawPolicy::PencilOnly } else { DrawPolicy::None }
}

/// Off iOS this shell's code is only compiled and tested: every other platform draws with any input.
#[cfg(not(target_os = "ios"))]
pub fn device_policy() -> DrawPolicy {
    DrawPolicy::AnyInput
}

/// The press egui's pointer holds, per the events that reached it.
#[derive(Debug, Clone, Copy)]
struct Press {
    /// The touch whose press it is.
    id: TouchId,
    pos: Pos2,
    /// Not a finger: the pencil, or a mouse (which reports no touch at all).
    pencil: bool,
    /// Written by [`PencilPointer`], not egui-winit: the pencil landed while a finger owned the
    /// pointer.
    injected: bool,
    /// The press began on the canvas itself, not on a control over it.
    canvas: bool,
}

/// The pencil's pointer where only the pencil draws. Feed it each frame's events, in order, as
/// egui-winit wrote them: a touch's pointer events follow its `Touch` event.
#[derive(Debug, Default)]
pub struct PencilPointer {
    /// The touch egui-winit's pointer follows: the first that started, until it ends or is
    /// cancelled (its `pointer_touch_id`).
    owner: Option<TouchId>,
    pressed: Option<Press>,
    /// The finger whose own pointer events are dropped until it ends: its press was ended for
    /// the pencil's, so its later release would be one more than egui's pointer had presses.
    dropped: Option<TouchId>,
    /// `dropped` ended: its last pointer events (the release egui-winit writes for an ending
    /// owner) follow its `Touch` event, so it goes after those.
    dropped_ended: bool,
    /// The touch whose `Touch` event the pointer events that follow belong to, when it owns the
    /// pointer; and whether that touch is the pencil's.
    belongs: Option<TouchId>,
    belongs_pencil: bool,
}

fn press(pos: Pos2) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }
}

fn release(pos: Pos2) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Default::default() }
}

impl PencilPointer {
    /// Rewrite one frame of events in place. `pencil_only`: the active tool draws and the policy
    /// is the pencil's alone; otherwise every event passes through untouched. `on_canvas`: a press
    /// there is the canvas's own, not a panel's, menu's or dialog's over it.
    pub fn frame(&mut self, events: &mut Vec<Event>, pencil_only: bool, on_canvas: &impl Fn(Pos2) -> bool) {
        let mut out = Vec::with_capacity(events.len());
        for ev in events.drain(..) {
            let Event::Touch { id, phase, pos, force, .. } = ev else {
                self.pointer(ev, &mut out, on_canvas);
                continue;
            };
            let pencil = force.is_some();
            if self.dropped_ended {
                self.dropped = None;
                self.dropped_ended = false;
            }
            self.belongs = (self.owner == Some(id)).then_some(id);
            self.belongs_pencil = pencil;
            out.push(ev);
            match phase {
                TouchPhase::Start => {
                    if self.owner.is_none() {
                        self.owner = Some(id);
                        self.belongs = Some(id);
                    }
                    if !pencil_only || !pencil {
                        continue;
                    }
                    // A finger's press is holding egui's pointer. On the canvas it was a drawing
                    // gesture the policy refused, so it ends here; on a control the finger is
                    // still using (a button, a slider) it keeps the pointer instead.
                    if let Some(p) = self.pressed.filter(|p| !p.pencil && p.canvas) {
                        out.push(release(p.pos));
                        out.push(Event::PointerGone);
                        self.pressed = None;
                        self.dropped = (self.owner == Some(p.id)).then_some(p.id);
                    }
                    // A finger still owns egui-winit's pointer: it writes nothing for the pencil.
                    if self.owner != Some(id) && self.pressed.is_none() {
                        out.push(Event::PointerMoved(pos));
                        out.push(press(pos));
                        self.pressed = Some(Press { id, pos, pencil, injected: true, canvas: true });
                    }
                }
                TouchPhase::Move => {
                    if let Some(p) = self.pressed.as_mut().filter(|p| p.injected && p.id == id) {
                        p.pos = pos;
                        out.push(Event::PointerMoved(pos));
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => {
                    if let Some(p) = self.pressed.filter(|p| p.id == id) {
                        // The pencil's own press ends with it. So does a cancelled canvas press:
                        // egui-winit releases only one that ends, and a pointer left down would
                        // take the pencil's next press for a drag already under way. A press that
                        // ends is released by the event egui-winit writes next, and one on a
                        // control keeps the platform's own behaviour.
                        if p.injected || (phase == TouchPhase::Cancel && p.canvas) {
                            out.push(release(pos));
                        }
                        if p.injected {
                            out.push(Event::PointerGone);
                        }
                        self.pressed = None;
                    }
                    if self.owner == Some(id) {
                        self.owner = None;
                    }
                    if self.dropped == Some(id) {
                        self.dropped_ended = true;
                    }
                }
            }
        }
        *events = out;
    }

    /// A pointer event egui-winit wrote: kept or dropped, and what it leaves the pointer as.
    fn pointer(&mut self, ev: Event, out: &mut Vec<Event>, on_canvas: &impl Fn(Pos2) -> bool) {
        // While the pencil's press is this one's, its pointer stream is too (a touch that owns
        // nothing still moves egui-winit's pointer, a resting hand included).
        if self.pressed.is_some_and(|p| p.injected) {
            return;
        }
        if self.belongs.is_some() && self.belongs == self.dropped {
            return;
        }
        let touch = self.belongs;
        match ev {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, .. } => {
                self.pressed = Some(Press {
                    id: touch.unwrap_or(TouchId(0)),
                    pos,
                    pencil: touch.is_none_or(|_| self.belongs_pencil),
                    injected: false,
                    canvas: on_canvas(pos),
                });
            }
            Event::PointerButton { button: PointerButton::Primary, pressed: false, .. } => self.pressed = None,
            Event::PointerMoved(pos) => {
                if let Some(p) = self.pressed.as_mut().filter(|p| touch.is_none_or(|t| t == p.id)) {
                    p.pos = pos;
                }
            }
            _ => {}
        }
        out.push(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Modifiers, TouchDeviceId, pos2, vec2};

    fn touch(id: u64, phase: TouchPhase, pos: Pos2, pencil: bool) -> Event {
        Event::Touch { device_id: TouchDeviceId(1), id: TouchId(id), phase, pos, force: pencil.then_some(0.4) }
    }
    /// What egui-winit writes for the touch that owns its pointer: its `Touch` event, then the
    /// pointer events it emulates from it.
    fn owned(touch_event: Event, pos: Pos2) -> Vec<Event> {
        let mut v = match touch_event {
            Event::Touch { phase: TouchPhase::Start, .. } => vec![Event::PointerMoved(pos), press(pos)],
            Event::Touch { phase: TouchPhase::Move, .. } => vec![Event::PointerMoved(pos)],
            Event::Touch { phase: TouchPhase::End, .. } => vec![release(pos), Event::PointerGone],
            _ => vec![Event::PointerGone],
        };
        v.insert(0, touch_event);
        v
    }
    /// The pointer stream: moves, presses (+) and releases (-) with their positions, and gone.
    fn pointer(events: &[Event]) -> String {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PointerMoved(p) => Some(format!("m{},{}", p.x, p.y)),
                Event::PointerButton { button: PointerButton::Primary, pressed: true, pos, .. } => Some(format!("+{},{}", pos.x, pos.y)),
                Event::PointerButton { button: PointerButton::Primary, pressed: false, pos, .. } => Some(format!("-{},{}", pos.x, pos.y)),
                Event::PointerGone => Some("gone".into()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    /// The touch stream: id, pencil (p) or finger (f), start (^) move (~) end ($) cancel (x).
    fn touches(events: &[Event]) -> String {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Touch { id, phase, force, .. } => {
                    let phase = match phase {
                        TouchPhase::Start => '^',
                        TouchPhase::Move => '~',
                        TouchPhase::End => '$',
                        TouchPhase::Cancel => 'x',
                    };
                    Some(format!("{}{}{}", id.0, if force.is_some() { "p" } else { "f" }, phase))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    const YES: fn(Pos2) -> bool = |_| true;

    #[test]
    fn with_the_policy_off_nothing_changes() {
        let a = pos2(100.0, 100.0);
        let b = a + vec2(20.0, 0.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        for frame in
            [owned(touch(1, TouchPhase::Start, a, false), a), owned(touch(1, TouchPhase::Move, b, false), b), owned(touch(1, TouchPhase::End, b, false), b)]
        {
            let mut events = frame;
            pp.frame(&mut events, false, &YES);
            all.extend(events);
        }
        assert_eq!(pointer(&all), "m100,100 +100,100 m120,100 -120,100 gone", "every pointer event as egui-winit wrote it");
        assert_eq!(touches(&all), "1f^ 1f~ 1f$");
    }

    #[test]
    fn the_pencil_alone_owns_its_pointer_as_before() {
        let a = pos2(100.0, 100.0);
        let b = a + vec2(20.0, 0.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        for frame in
            [owned(touch(1, TouchPhase::Start, a, true), a), owned(touch(1, TouchPhase::Move, b, true), b), owned(touch(1, TouchPhase::End, b, true), b)]
        {
            let mut events = frame;
            pp.frame(&mut events, true, &YES);
            all.extend(events);
        }
        assert_eq!(pointer(&all), "m100,100 +100,100 m120,100 -120,100 gone");
    }

    /// A finger lands first, then the pencil: the finger's press ends where it was, and the
    /// stroke begins where the pencil lands and runs to where it lifts.
    #[test]
    fn a_pencil_landing_on_a_fingers_press_takes_the_pointer() {
        let f = pos2(100.0, 100.0);
        let (a, b) = (pos2(150.0, 150.0), pos2(170.0, 150.0));
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        let feed = |pp: &mut PencilPointer, all: &mut Vec<Event>, events: Vec<Event>| {
            let mut events = events;
            pp.frame(&mut events, true, &YES);
            all.extend(events);
        };
        // Frame 1: the finger's press (refused by the policy as a stroke). Frame 2: iPadOS
        // cancels the finger as the pencil lands, which now owns egui-winit's pointer. Frames 3
        // and 4: the pencil draws, then lifts.
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Start, f, false), f));
        feed(
            &mut pp,
            &mut all,
            vec![touch(1, TouchPhase::Cancel, f, false), Event::PointerGone, touch(2, TouchPhase::Start, a, true), Event::PointerMoved(a), press(a)],
        );
        feed(&mut pp, &mut all, owned(touch(2, TouchPhase::Move, b, true), b));
        feed(&mut pp, &mut all, owned(touch(2, TouchPhase::End, b, true), b));
        assert_eq!(
            pointer(&all),
            "m100,100 +100,100 -100,100 gone m150,150 +150,150 m170,150 -170,150 gone",
            "one press for the finger, ended; one for the pencil, whole"
        );
        assert_eq!(touches(&all), "1f^ 1fx 2p^ 2p~ 2p$", "every touch event passes through");
    }

    /// Without the cancellation the finger owns egui-winit's pointer until it lifts: the pencil's
    /// pointer events are written here, and the finger's release, arriving after the pencil's,
    /// is dropped.
    #[test]
    fn a_resting_finger_keeps_the_ownership_and_loses_its_own_release() {
        let f = pos2(100.0, 100.0);
        let (a, b) = (pos2(150.0, 150.0), pos2(170.0, 150.0));
        let g = pos2(105.0, 100.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        let feed = |pp: &mut PencilPointer, all: &mut Vec<Event>, events: Vec<Event>| {
            let mut events = events;
            pp.frame(&mut events, true, &YES);
            all.extend(events);
        };
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Start, f, false), f));
        feed(&mut pp, &mut all, vec![touch(2, TouchPhase::Start, a, true)]);
        // The finger's move would drag egui-winit's pointer to it.
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Move, g, false), g));
        feed(&mut pp, &mut all, vec![touch(2, TouchPhase::Move, b, true), touch(2, TouchPhase::End, b, true)]);
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::End, g, false), g));
        assert_eq!(pointer(&all), "m100,100 +100,100 -100,100 gone m150,150 +150,150 m170,150 -170,150 gone", "the pencil's stroke, and no second release");
        // The finger lifted: the next touch is anyone's, and passes through as before.
        feed(&mut pp, &mut all, owned(touch(3, TouchPhase::Start, g, false), g));
        assert!(pointer(&all).ends_with("m105,100 +105,100"));
    }

    /// The resting hand joining a stroke: it owns nothing and changes nothing.
    #[test]
    fn a_finger_joining_the_pencil_is_ignored() {
        let (a, b, c) = (pos2(100.0, 100.0), pos2(120.0, 100.0), pos2(140.0, 100.0));
        let hand = pos2(300.0, 300.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        let feed = |pp: &mut PencilPointer, all: &mut Vec<Event>, events: Vec<Event>| {
            let mut events = events;
            pp.frame(&mut events, true, &YES);
            all.extend(events);
        };
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Start, a, true), a));
        feed(&mut pp, &mut all, vec![touch(2, TouchPhase::Start, hand, false), touch(1, TouchPhase::Move, b, true), Event::PointerMoved(b)]);
        feed(&mut pp, &mut all, vec![touch(2, TouchPhase::End, hand, false)]);
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Move, c, true), c));
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::End, c, true), c));
        assert_eq!(pointer(&all), "m100,100 +100,100 m120,100 m140,100 -140,100 gone");
        assert_eq!(touches(&all), "1p^ 2f^ 1p~ 2f$ 1p~ 1p$");
    }

    /// The pencil's own press ends when iPadOS cancels it: egui-winit writes no release then,
    /// and one is written here.
    #[test]
    fn a_cancelled_pencil_press_is_released() {
        let a = pos2(100.0, 100.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        for frame in [owned(touch(1, TouchPhase::Start, a, true), a), vec![touch(1, TouchPhase::Cancel, a, true), Event::PointerGone]] {
            let mut events = frame;
            pp.frame(&mut events, true, &YES);
            all.extend(events);
        }
        assert_eq!(pointer(&all), "m100,100 +100,100 -100,100 gone");
    }

    /// A cancelled press on a control is left to the platform as before: only the canvas's are
    /// released here.
    #[test]
    fn a_cancelled_press_on_a_control_is_not_released() {
        let at = pos2(100.0, 100.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        for frame in [owned(touch(1, TouchPhase::Start, at, false), at), vec![touch(1, TouchPhase::Cancel, at, false), Event::PointerGone]] {
            let mut events = frame;
            pp.frame(&mut events, true, &|p| p.x > 120.0);
            all.extend(events);
        }
        assert_eq!(pointer(&all), "m100,100 +100,100 gone", "no release of egui-winit's own is added");
    }

    /// A finger holding a control keeps it: the pencil lands on the canvas and owns nothing
    /// until the finger lifts, as when the policy is off.
    #[test]
    fn a_finger_press_on_a_control_is_not_taken() {
        let f = pos2(100.0, 100.0);
        let a = pos2(150.0, 150.0);
        let mut pp = PencilPointer::default();
        let mut all = Vec::new();
        let feed = |pp: &mut PencilPointer, all: &mut Vec<Event>, events: Vec<Event>| {
            let mut events = events;
            pp.frame(&mut events, true, &|p| p.x > 120.0);
            all.extend(events);
        };
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::Start, f, false), f));
        feed(&mut pp, &mut all, vec![touch(2, TouchPhase::Start, a, true), touch(2, TouchPhase::End, a, true)]);
        feed(&mut pp, &mut all, owned(touch(1, TouchPhase::End, f, false), f));
        assert_eq!(pointer(&all), "m100,100 +100,100 -100,100 gone", "the finger's press, and only it");
        assert_eq!(touches(&all), "1f^ 2p^ 2p$ 1f$");
    }

    /// Both land in one frame: the finger's press goes through and ends in the same batch, the
    /// pencil's begins in it.
    #[test]
    fn a_pencil_landing_in_the_frame_a_finger_pressed() {
        let (f, a) = (pos2(100.0, 100.0), pos2(150.0, 150.0));
        let mut pp = PencilPointer::default();
        let mut events = owned(touch(1, TouchPhase::Start, f, false), f);
        events.push(touch(2, TouchPhase::Start, a, true));
        pp.frame(&mut events, true, &YES);
        assert_eq!(pointer(&events), "m100,100 +100,100 -100,100 gone m150,150 +150,150");
    }

    /// A mouse or trackpad's buttons (which report no touch) pass through, and leave no press
    /// a later pencil landing would end.
    #[test]
    fn a_mouse_press_passes_through_and_is_a_finger_to_no_one() {
        let at = pos2(5.0, 5.0);
        let mut pp = PencilPointer::default();
        let mut events =
            vec![press(at), release(at), Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE }];
        let before = events.clone();
        pp.frame(&mut events, true, &YES);
        assert_eq!(format!("{events:?}"), format!("{before:?}"));
    }
}
