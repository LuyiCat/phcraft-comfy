//! Testing aid for the iPad's drawing policy (development builds only, see `main.rs`):
//! `PHOTOCRAFT_DRAW_PROBE` replays finger and Apple Pencil touches on the canvas, written exactly
//! as egui-winit delivers them (a touch's pointer events follow its `Touch` event), and logs what
//! each one did to `Documents/launch.log`: the strokes committed, the selection left, the hint.
//! With no finger or pencil to be had it is how a simulator run shows that a finger never draws,
//! fills or selects, that the pencil does, and that neither the resting hand nor a finger that
//! held the pointer first keeps the pencil from a stroke.

use egui::{Event, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase, vec2};
use photocraft_ui_egui::{PhotocraftApp, Tool};

/// The frame the document opens on. The gestures begin a few frames later, once the canvas has
/// drawn it and its rect says where it is, and are spaced so each one's gesture commits.
pub(super) const START_FRAME: u32 = 30;
const PLACE_FRAMES: u32 = 4;
const STEP_FRAMES: u32 = 4;

enum Step {
    /// The frame's events, as egui-winit writes them, put before the frame's own.
    Events(Vec<Event>),
    Tool(Tool),
    /// What the gesture just replayed did on the iPad: how many history entries it committed, and
    /// whether it left a selection. On the iPhone nothing may commit anything.
    Check {
        name: &'static str,
        strokes: i64,
        selection: bool,
    },
}

pub(super) struct Probe {
    /// The gestures, placed once the canvas has drawn the document.
    steps: Option<std::collections::VecDeque<Step>>,
    strokes: usize,
}

fn touch(id: u64, phase: TouchPhase, pos: Pos2, pencil: bool) -> Event {
    Event::Touch { device_id: TouchDeviceId(1), id: TouchId(id), phase, pos, force: pencil.then_some(0.5) }
}

/// egui-winit's pointer events for the touch that owns its pointer.
fn pointer(phase: TouchPhase, pos: Pos2) -> Vec<Event> {
    let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() };
    match phase {
        TouchPhase::Start => vec![Event::PointerMoved(pos), button(true)],
        TouchPhase::Move => vec![Event::PointerMoved(pos)],
        TouchPhase::End => vec![button(false), Event::PointerGone],
        TouchPhase::Cancel => vec![Event::PointerGone],
    }
}

/// A contact that owns the pointer: its touch, then the pointer events egui-winit emulates from it.
fn owned(ev: Event, phase: TouchPhase, pos: Pos2) -> Vec<Event> {
    [vec![ev], pointer(phase, pos)].concat()
}

impl Probe {
    /// The document the gestures draw on.
    fn new(app: &mut PhotocraftApp) -> Self {
        use serde_json::json;
        let note = |r: Result<serde_json::Value, String>| {
            if let Err(e) = r {
                super::breadcrumb(&format!("draw probe: setup: {e}"));
            }
        };
        note(app.run("file.new", json!({"width": 256, "height": 256, "background": "white"})));
        note(app.run("layer.new.layer", json!({})));
        note(app.run("tools.setBrush", json!({"brush": {"size": 12, "hardness": 1.0, "smoothing": {"amount": 0.0}}})));
        note(app.run("tools.setColors", json!({"foreground": [0.0, 0.0, 0.0, 1.0]})));
        app.sync_views();
        Self { steps: None, strokes: 0 }
    }

    /// Lay the gestures out where the canvas shows the document, a few frames on.
    fn place(&mut self, app: &mut PhotocraftApp) {
        let c = app.last_canvas_rect.center();
        let (l, r, hand, low) = (c - vec2(30.0, 0.0), c + vec2(30.0, 0.0), c + vec2(5.0, 90.0), c + vec2(0.0, 20.0));
        let frames = |v: Vec<Vec<Event>>| v.into_iter().map(Step::Events).collect::<Vec<_>>();
        let nobody = app.draw_policy == photocraft_ui_egui::draw_policy::DrawPolicy::None;
        let check = |name: &'static str, strokes, selection| Step::Check { name, strokes: if nobody { 0 } else { strokes }, selection: selection && !nobody };
        // A drag from `l` to `r` by one contact that owns the pointer throughout.
        let drag = |id: u64, pencil: bool| {
            vec![
                owned(touch(id, TouchPhase::Start, l, pencil), TouchPhase::Start, l),
                owned(touch(id, TouchPhase::Move, c, pencil), TouchPhase::Move, c),
                owned(touch(id, TouchPhase::End, r, pencil), TouchPhase::End, r),
            ]
        };
        let mut steps: std::collections::VecDeque<Step> = Default::default();
        steps.push_back(Step::Tool(Tool::Brush));
        steps.extend(frames(drag(1, false)));
        steps.push_back(check("brush, finger", 0, false));
        steps.extend(frames(drag(2, true)));
        steps.push_back(check("brush, pencil", 1, false));
        // A finger holds the pointer; iPadOS cancels it as the pencil lands, in one frame.
        steps.extend(frames(vec![
            owned(touch(3, TouchPhase::Start, l, false), TouchPhase::Start, l),
            // The cancel frees egui-winit's pointer, so the pencil's landing owns it again.
            [vec![touch(3, TouchPhase::Cancel, l, false), Event::PointerGone], owned(touch(4, TouchPhase::Start, l, true), TouchPhase::Start, l)].concat(),
            owned(touch(4, TouchPhase::Move, c, true), TouchPhase::Move, c),
            owned(touch(4, TouchPhase::End, r, true), TouchPhase::End, r),
        ]));
        steps.push_back(check("brush, finger holding the pointer and cancelled, then pencil", 1, false));
        // A finger holds the pointer and stays down: egui-winit writes nothing for the pencil,
        // and later the finger's own release.
        steps.extend(frames(vec![
            owned(touch(5, TouchPhase::Start, l, false), TouchPhase::Start, l),
            vec![touch(6, TouchPhase::Start, l, true)],
            vec![touch(6, TouchPhase::Move, c, true)],
            vec![touch(6, TouchPhase::End, r, true)],
            owned(touch(5, TouchPhase::End, l, false), TouchPhase::End, l),
        ]));
        steps.push_back(check("brush, finger holding the pointer and staying, then pencil", 1, false));
        // The pencil draws; the resting hand lands beside the stroke and lifts.
        steps.extend(frames(vec![
            owned(touch(7, TouchPhase::Start, l, true), TouchPhase::Start, l),
            vec![touch(8, TouchPhase::Start, hand, false)],
            owned(touch(7, TouchPhase::Move, c, true), TouchPhase::Move, c),
            vec![touch(8, TouchPhase::End, hand, false)],
            owned(touch(7, TouchPhase::End, r, true), TouchPhase::End, r),
        ]));
        steps.push_back(check("brush, the resting hand joining the stroke", 1, false));
        steps.push_back(Step::Tool(Tool::RectMarquee));
        steps.extend(frames(drag(9, false)));
        steps.push_back(check("marquee, finger", 0, false));
        steps.extend(frames(drag(10, true)));
        // A marquee commits its selection as one history entry.
        steps.push_back(check("marquee, pencil", 1, true));
        let tap = |id: u64, at, pencil| {
            vec![owned(touch(id, TouchPhase::Start, at, pencil), TouchPhase::Start, at), owned(touch(id, TouchPhase::End, at, pencil), TouchPhase::End, at)]
        };
        steps.push_back(Step::Tool(Tool::PaintBucket));
        steps.extend(frames(tap(11, c, false)));
        steps.push_back(check("paint bucket, finger", 0, false));
        steps.extend(frames(tap(12, low, true)));
        steps.push_back(check("paint bucket, pencil", 1, false));
        self.strokes = strokes(app);
        super::breadcrumb(&format!(
            "draw probe: policy {:?}, canvas {:?} with the 256 px document fitted; each line's expectations are what the gesture does under that policy",
            app.draw_policy, app.last_canvas_rect
        ));
        self.steps = Some(steps);
    }
}

fn strokes(app: &PhotocraftApp) -> usize {
    app.session.active().map_or(0, |st| st.history.past_len())
}

/// At the top of the frame's input: open the document on the first frame, lay the gestures out
/// once the canvas has drawn it, then feed a step every few frames.
pub(super) fn inject(app: &mut PhotocraftApp, frame: u32, probe: &mut Option<Probe>, raw: &mut egui::RawInput) {
    if frame == START_FRAME {
        *probe = Some(Probe::new(app));
    }
    let Some(p) = probe.as_mut() else { return };
    if frame == START_FRAME + PLACE_FRAMES {
        p.place(app);
    }
    let Some(steps) = p.steps.as_mut() else { return };
    let first = START_FRAME + PLACE_FRAMES + STEP_FRAMES;
    if frame < first || (frame - first) % STEP_FRAMES != 0 {
        return;
    }
    match steps.pop_front() {
        Some(Step::Events(events)) => {
            let mut all = events;
            all.extend(raw.events.drain(..));
            raw.events = all;
        }
        Some(Step::Tool(tool)) => {
            // Set directly: on the iPhone, where no drawing tool may be selected, the canvas
            // refusing the gesture is the point.
            app.ui.tool = tool;
        }
        Some(Step::Check { name, strokes: want, selection }) => {
            let now = strokes(app);
            let left = app.session.active().is_some_and(|st| st.doc.selection.is_some());
            let hint = app.ui.status.clone();
            let ok = now as i64 - p.strokes as i64 == want && left == selection;
            let bounds = app.session.active().and_then(|st| st.doc.selection.as_ref()).map(|s| format!("{:?}", s.content_bounds())).unwrap_or_default();
            super::breadcrumb(&format!(
                "draw probe: {name}: committed {} (expected {want}), selection {left} (expected {selection}) {bounds}, hint {hint:?}: {}",
                now as i64 - p.strokes as i64,
                if ok { "as expected" } else { "UNEXPECTED" }
            ));
            // Clear what the gesture left, so the next one starts from the same state; a refusal
            // of the clearing is no message worth keeping.
            let _ = app.run("select.deselect", serde_json::json!({}));
            app.ui.status = hint;
            p.strokes = strokes(app);
        }
        None => {}
    }
}
