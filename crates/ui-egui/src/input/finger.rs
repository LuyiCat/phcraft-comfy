//! Which touch contact the emulated pointer press belongs to, and whether it is a finger.
//!
//! winit emulates the mouse pointer from touch contacts: the contact that started last owns the
//! press — an owning contact's End frees the pointer for the next contact that starts, and
//! iPadOS cancels a resting finger when the Apple Pencil lands, so after any cancellation the
//! newest contact is the pen. [`FingerPresses`] follows the frame's events in order (from
//! `logic_frame`, before the canvas runs; kittest harnesses reach it too, since they skip
//! `raw_input_hook`) to answer two questions. Is the emulated primary press in progress a
//! finger's — judged at its press, from the contact that started last, exactly as
//! [`crate::stylus::Stylus::update`] judges the raw force, before any pen-pressure-0 mapping
//! could lend a known pen a force? And did a finger's press end this frame — for egui reports a
//! click on the release, and the click tools (the Paint Bucket, the Magic Eraser, the wand) run
//! then, in the frame the finger has already lifted and a pencil landing in the same frame has
//! already begun its own press. A finger is a contact that began without a force — the Apple
//! Pencil's contacts always carry one, and no iPad ever had 3D Touch — while no contact at all
//! (a mouse, `ui.pointer` automation) is never a finger.
//!
//! This is the drawing policy's classifier (`PhotocraftApp::draw_policy`, input/draw_policy.rs):
//! a resting finger after the long-press menu must not steal a later pencil press's
//! classification, which "any finger is down" ([`crate::stylus::Stylus::finger_down`]) would.

/// The emulated pointer press, attributed. Feed it one frame's events, in order.
#[derive(Debug, Default)]
pub struct FingerPresses {
    /// The contacts currently down that began without a force: fingers.
    fingers: Vec<egui::TouchId>,
    /// The contact that started last and is still down (`None` once it lifts, until the next
    /// contact starts).
    latest: Option<egui::TouchId>,
    /// The emulated primary press in progress, and whether a finger began it.
    press: Option<bool>,
    /// A finger's press ended this frame: the click egui reports on that release is the
    /// finger's, whatever press began in the same frame.
    finger_released: bool,
    /// The egui frame [`Self::update_for_frame`] last ran in.
    fed_frame: Option<u64>,
}

/// Most touch ids remembered (the oldest go first, as everywhere else in the input state).
const CONTACTS: usize = 16;

impl FingerPresses {
    /// [`Self::frame`] once per egui frame: a host can run a frame in several passes, and the
    /// same events must not be counted twice (the same guard `Stylus::update_for_frame` keeps).
    pub fn update_for_frame(&mut self, frame: u64, events: &[egui::Event]) {
        if self.fed_frame.replace(frame) != Some(frame) {
            self.frame(events);
        }
    }

    /// Follow one frame's events: the touches, and the primary presses and releases egui-winit
    /// emulates from them (or a shell rewrites for the pencil, `apps/photocraft-ios/src/drawing.rs`).
    pub fn frame(&mut self, events: &[egui::Event]) {
        self.finger_released = false;
        for e in events {
            match *e {
                egui::Event::Touch { id, phase, force, .. } => match phase {
                    egui::TouchPhase::Start => {
                        if force.filter(|f| f.is_finite()).is_some() {
                            self.fingers.retain(|f| *f != id);
                        } else if !self.fingers.contains(&id) {
                            if self.fingers.len() >= CONTACTS {
                                self.fingers.remove(0);
                            }
                            self.fingers.push(id);
                        }
                        self.latest = Some(id);
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                        // The finger that held the pointer lifted (iPadOS may send no release for
                        // a cancelled one): its press is over, its click this frame is its own.
                        if self.latest == Some(id) {
                            if self.press == Some(true) {
                                self.finger_released = true;
                                self.press = None;
                            }
                            self.latest = None;
                        }
                        self.fingers.retain(|f| *f != id);
                    }
                    egui::TouchPhase::Move => {}
                },
                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: true, .. } => {
                    self.press = Some(self.latest.is_some_and(|id| self.fingers.contains(&id)));
                }
                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: false, .. } => {
                    self.finger_released |= self.press == Some(true);
                    self.press = None;
                }
                _ => {}
            }
        }
    }

    /// Is the emulated primary press in progress a finger's? A mouse, `ui.pointer` automation
    /// and any force-reporting touch (the pencil) are not.
    pub fn pointer_is_finger(&self) -> bool {
        self.press == Some(true)
    }

    /// Did a finger's press end this frame? The click egui reports on that release is the
    /// finger's, even when a pencil began its own press in the same frame.
    pub fn finger_released(&self) -> bool {
        self.finger_released
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Modifiers, Pos2, TouchDeviceId, TouchId, TouchPhase};

    fn touch(id: u64, phase: TouchPhase, force: Option<f32>) -> egui::Event {
        egui::Event::Touch { device_id: TouchDeviceId(1), id: TouchId(id), phase, pos: Pos2::new(10.0, 10.0), force }
    }
    fn button(pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos: Pos2::new(10.0, 10.0), button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    #[test]
    fn a_contact_without_a_force_is_a_finger_a_force_is_the_pencil() {
        let mut f = FingerPresses::default();
        f.frame(&[touch(1, TouchPhase::Start, None)]);
        f.frame(&[button(true)]);
        assert!(f.pointer_is_finger(), "a finger began and owns the press");
        f.frame(&[button(false)]);
        assert!(!f.pointer_is_finger(), "the lift frees the pointer");
        assert!(f.finger_released(), "the click on that release is the finger's");
        f.frame(&[]);
        assert!(!f.finger_released(), "for that frame only");
        f.frame(&[touch(2, TouchPhase::Start, Some(0.0)), button(true)]);
        assert!(!f.pointer_is_finger(), "the pencil reports a force, even 0.0");
    }

    #[test]
    fn the_contact_that_started_last_owns_the_press() {
        let mut f = FingerPresses::default();
        // A finger rests (its long-press menu's release has come and gone); the pencil lands
        // and iPadOS cancels the finger.
        f.frame(&[touch(1, TouchPhase::Start, None), button(true), button(false)]);
        f.frame(&[touch(2, TouchPhase::Start, Some(0.3)), touch(1, TouchPhase::Cancel, None), button(true)]);
        assert!(!f.pointer_is_finger(), "the resting finger did not steal the pencil's press");
        // Without the cancellation the same holds: the newest contact is the owner.
        let mut f = FingerPresses::default();
        f.frame(&[touch(1, TouchPhase::Start, None), touch(2, TouchPhase::Start, Some(0.3)), button(true)]);
        assert!(!f.pointer_is_finger());
        // The pencil lifts: nothing owns the press, finger still down or not.
        f.frame(&[touch(2, TouchPhase::End, Some(0.3)), button(false)]);
        assert!(!f.pointer_is_finger());
        assert!(!f.finger_released(), "the pencil's release is no finger's click");
        f.frame(&[touch(3, TouchPhase::Start, None)]);
        f.frame(&[button(true)]);
        assert!(f.pointer_is_finger(), "the next finger that starts owns the press again");
    }

    #[test]
    fn no_contact_at_all_is_never_a_finger() {
        let f = FingerPresses::default();
        assert!(!f.pointer_is_finger(), "a mouse or ui.pointer automation paints");
        let mut f = FingerPresses::default();
        f.frame(&[button(true)]);
        assert!(!f.pointer_is_finger());
        f.frame(&[egui::Event::PointerMoved(Pos2::new(5.0, 5.0))]);
        assert!(!f.pointer_is_finger());
    }

    /// The shell ends a finger's canvas press when the pencil lands, and begins the pencil's in
    /// the same frame: the finger's click on that release stays the finger's (refused), the
    /// pencil's press is the pencil's (it draws).
    #[test]
    fn a_fingers_press_ended_for_the_pencil_stays_the_fingers_through_the_frame() {
        let mut f = FingerPresses::default();
        f.frame(&[touch(1, TouchPhase::Start, None), button(true)]);
        assert!(f.pointer_is_finger());
        f.frame(&[touch(2, TouchPhase::Start, Some(0.4)), button(false), button(true)]);
        assert!(f.finger_released(), "the release was the finger's");
        assert!(!f.pointer_is_finger(), "the new press is the pencil's");
        f.frame(&[]);
        assert!(!f.finger_released());
        // A cancelled finger's press is over too, though egui-winit writes no release for it.
        let mut f = FingerPresses::default();
        f.frame(&[touch(1, TouchPhase::Start, None), button(true)]);
        f.frame(&[touch(1, TouchPhase::Cancel, None)]);
        assert!(!f.pointer_is_finger());
        assert!(f.finger_released());
    }
}
