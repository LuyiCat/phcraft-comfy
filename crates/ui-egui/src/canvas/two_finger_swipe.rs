//! Two fingers flicked across the canvas: Undo (left) / Redo (right), the touch answer to ⌘Z/⌘⇧Z
//! that the quick-actions column (`panels/quick_actions.rs`) also carries. Enhanced Controls ›
//! Enable Gestures gates it as it does the two-finger pan, pinch and twist (#204).
//!
//! Only a flick counts: two fingers that set off sideways the moment the second one lands. Every
//! other stroke stays navigation, and a stroke once judged a pan never turns into a swipe. Until
//! the judgement the fingers pan and pinch as usual; a swipe then puts the camera back where it
//! was when they landed, and until they lift they navigate nothing.

use egui::{MultiTouchInfo, Pos2};

/// Sideways travel, in points, at which a stroke is judged.
const TRAVEL: f32 = 60.0;
/// How fast, in points per second from its first movement, the stroke must have covered `TRAVEL`.
const SPEED: f32 = 600.0;
/// A stroke at least this many times wider than tall is sideways (22° off the horizontal).
const SIDEWAYS: f32 = 2.5;
/// How far the fingers' pinch may wander while they travel (1.0 is no pinch).
const SPREAD: (f32, f32) = (0.85, 1.18);
/// Seconds after the second finger lands in which the fingers must set off; a stroke that
/// waits is a pan.
const START_DELAY: f64 = 0.3;
/// Travel, in points, that counts as setting off.
const SET_OFF: f32 = 2.0;

/// The camera of a canvas: zoom, centre, rotation.
type Camera = (f32, [f32; 2], f32);

/// egui's identity for a gesture (`MultiTouchInfo::start_time` and `start_pos`): fingers that
/// part and land again are a new one.
type GestureId = (f64, Pos2);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Verdict {
    /// Still travelling: a swipe or a pan.
    Pending,
    /// A flick, left (`true`) or right.
    Swiped(bool),
    Panned,
}

/// The canvas that took the gesture, and what it puts back if the stroke is a swipe.
#[derive(Clone, Copy, Debug)]
struct Claim {
    canvas: usize,
    camera: Camera,
    /// The fingers' centre on the frame of the claim. Not egui's `start_pos`: that is wherever
    /// the pointer was the frame before, and nothing at all when no pointer has been reported.
    origin: Pos2,
}

#[derive(Clone, Copy, Debug)]
struct Gesture {
    id: GestureId,
    /// When the gesture was first seen: a canvas may claim it only on that frame.
    first_seen: f64,
    /// `None` for as long as no canvas has claimed it.
    claim: Option<Claim>,
    /// The previous frame's time.
    seen: f64,
    /// The previous frame's time, once the centre has set off from the claim's origin.
    moving_since: Option<f64>,
    /// The fingers' pinch since the claim (1.0 is none).
    spread: f32,
    verdict: Verdict,
}

impl Gesture {
    fn new(id: GestureId, now: f64) -> Self {
        Self { id, first_seen: now, claim: None, seen: now, moving_since: None, spread: 1.0, verdict: Verdict::Pending }
    }

    /// Follows the claimed stroke one frame further; true on the frame it is judged a swipe.
    fn judge(&mut self, claim: Claim, m: &MultiTouchInfo, now: f64) -> bool {
        if self.verdict != Verdict::Pending {
            // Judged already. Fingers go on moving after a flick, out of step, and a pinch read
            // into that must not turn the swipe back into a pan, its navigation with it.
            return false;
        }
        let travel = m.center_pos - claim.origin;
        self.spread *= m.zoom_delta;
        if m.num_touches != 2 || !(SPREAD.0..=SPREAD.1).contains(&self.spread) {
            // A finger joining or leaving shifts the centre, and so does one finger staying put
            // through a pinch: neither is a flick.
            self.verdict = Verdict::Panned;
        } else if self.moving_since.is_none() {
            if travel.length_sq() >= SET_OFF * SET_OFF {
                self.moving_since = Some(self.seen);
            } else if now - m.start_time >= START_DELAY {
                self.verdict = Verdict::Panned;
            }
        }
        if self.verdict == Verdict::Pending
            && let Some(since) = self.moving_since
            && travel.x.abs() >= TRAVEL
        {
            // The shape is judged on the whole stroke: the ratio is noise over the first points.
            let sideways = travel.x.abs() >= SIDEWAYS * travel.y.abs();
            let fast = now > since && f64::from(travel.x.abs()) / (now - since) >= f64::from(SPEED);
            self.verdict = if sideways && fast { Verdict::Swiped(travel.x < 0.0) } else { Verdict::Panned };
            return true;
        }
        false
    }
}

/// What the frame's two-finger gesture means for the canvas it was read on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Reading {
    /// No swipe: the fingers navigate as usual.
    Navigate,
    /// This gesture already swiped: until the fingers lift they navigate nothing.
    Held,
    /// This frame judged the stroke a swipe (`left`): the camera goes back to `camera`.
    Swiped { left: bool, camera: Camera },
}

impl Reading {
    /// The fingers have swiped and are still down: until they lift, the canvas navigates nothing.
    pub(super) fn held(self) -> bool {
        matches!(self, Reading::Held | Reading::Swiped { .. })
    }
}

/// The two-finger gesture over the frames it runs (`update_canvas_navigation`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TwoFingerSwipe {
    gesture: Option<Gesture>,
}

impl TwoFingerSwipe {
    /// This frame's reading for canvas `canvas`, whose camera is `camera`. `gesture` is egui's
    /// multi-touch info with the frame's time (`None` with fewer than two fingers, or with
    /// gestures off). `began_here` says whether a position is on this canvas under nothing
    /// floating; it is asked only of a gesture's first frame.
    pub(super) fn read(&mut self, canvas: usize, camera: Camera, gesture: Option<(MultiTouchInfo, f64)>, began_here: impl FnOnce(Pos2) -> bool) -> Reading {
        let Some((m, now)) = gesture else {
            self.gesture = None;
            return Reading::Navigate;
        };
        let id = (m.start_time, m.start_pos);
        let g = match self.gesture {
            Some(ref mut g) if g.id == id => g,
            _ => self.gesture.insert(Gesture::new(id, now)),
        };
        let Some(claim) = g.claim.filter(|c| c.canvas == canvas) else {
            // Not this canvas's gesture. One that nobody has claimed may still be, but only on
            // the frame it began: a stroke that started over a panel or a window stays navigation
            // even once the fingers have moved onto a canvas.
            if g.claim.is_none() && g.first_seen == now && m.num_touches == 2 && began_here(m.center_pos) {
                g.claim = Some(Claim { canvas, camera, origin: m.center_pos });
            }
            return if matches!(g.verdict, Verdict::Swiped(_)) { Reading::Held } else { Reading::Navigate };
        };
        let swiped = g.judge(claim, &m, now);
        g.seen = now;
        match g.verdict {
            Verdict::Swiped(left) if swiped => Reading::Swiped { left, camera: claim.camera },
            Verdict::Swiped(_) => Reading::Held,
            Verdict::Pending | Verdict::Panned => Reading::Navigate,
        }
    }
}
