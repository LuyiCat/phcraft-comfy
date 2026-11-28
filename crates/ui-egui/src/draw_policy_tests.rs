//! The drawing policy (`PhotocraftApp::draw_policy`, input/draw_policy.rs): who may draw, fill
//! and select. A finger is a touch contact that began without a force, where the Apple Pencil's
//! always carries one; a mouse and automation report no contact at all.
//!
//! Under `PencilOnly` (the iPad) a finger never starts a stroke, a fill, a selection or a Liquify
//! push, with any drawing tool; the pencil, a mouse and automation draw as before, and every tool
//! that is no drawing keeps its touch behaviour. Under `None` (the iPhone) no press draws, the
//! tool commands offer no drawing tool and the menus no selection or fill. `AnyInput` — the
//! default everywhere else, Android included — is the old behaviour.

use egui::{Modifiers, Pos2, TouchDeviceId, TouchId, TouchPhase, pos2, vec2};
use egui_kittest::Harness;
use photocraft_algo::liquify::LiquifyTool;
use serde_json::json;

use crate::PhotocraftApp;
use crate::canvas::{ToolEvent, document_area};
use crate::draw_policy::DrawPolicy;
use crate::state::Tool;

const PENCIL_ONLY: DrawPolicy = DrawPolicy::PencilOnly;
const PENCIL_HINT: &str = "Painting needs the Apple Pencil";
const NONE_HINT: &str = "Drawing, filling and selecting are not available on this device";

/// The app over a transparent 64×64 document with a hard 8 px black brush, `policy` set and
/// `tool` selected.
fn app(policy: DrawPolicy, tool: Tool) -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", json!({"width": 64, "height": 64, "background": "transparent"})).unwrap();
    // An unlocked layer: the Background's position lock refuses a marquee before the tool runs.
    app.run("layer.new.layer", json!({})).unwrap();
    app.run("tools.setBrush", json!({"brush": {"size": 8, "hardness": 1.0, "smoothing": {"amount": 0.0}}})).unwrap();
    app.run("tools.setColors", json!({"foreground": [0.0, 0.0, 0.0, 1.0]})).unwrap();
    app.sync_views();
    app.set_draw_policy(policy);
    app.ui.tool = tool;
    app
}

/// The whole-app canvas harness (as the two-finger gesture tests build it).
fn canvas(app: PhotocraftApp) -> (Harness<'static, (PhotocraftApp, bool)>, Pos2) {
    let mut h = Harness::builder().with_size(egui::vec2(480.0, 360.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, (app, ready): &mut (PhotocraftApp, bool)| {
            if *ready {
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| document_area(app, ui));
            }
        },
        (app, false),
    );
    PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::Pro);
    h.state_mut().1 = true;
    h.run_steps(3);
    let centre = h.state().0.last_canvas_rect.center();
    (h, pos2(centre.x, centre.y))
}

fn touch(id: u64, phase: TouchPhase, pos: Pos2, force: Option<f32>) -> egui::Event {
    egui::Event::Touch { device_id: TouchDeviceId(0), id: TouchId(id), phase, pos, force }
}
fn press(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// One contact goes down at `at` — a finger (`force: None`) or the pencil.
fn contact_down(h: &mut Harness<'_, (PhotocraftApp, bool)>, at: Pos2, force: Option<f32>) {
    h.input_mut().events = vec![touch(1, TouchPhase::Start, at, force), press(at, true)];
    h.step();
    h.run_steps(1);
}

/// The contact moves and lifts.
fn contact_up(h: &mut Harness<'_, (PhotocraftApp, bool)>, to: Pos2, force: Option<f32>) {
    h.input_mut().events = vec![touch(1, TouchPhase::Move, to, force), egui::Event::PointerMoved(to)];
    h.step();
    h.input_mut().events = vec![touch(1, TouchPhase::End, to, force), press(to, false)];
    h.step();
    h.run_steps(2);
}

fn past_len(h: &Harness<'_, (PhotocraftApp, bool)>) -> usize {
    h.state().0.session.active().unwrap().history.past_len()
}

/// Every kind of drawing tool: the brush family and erasers, the fills, the selection tools,
/// the Pen and shape tools, a Type Mask.
const DRAWING_TOOLS: [Tool; 17] = [
    Tool::Brush,
    Tool::Eraser,
    Tool::Smudge,
    Tool::CloneStamp,
    Tool::HistoryBrush,
    Tool::MagicEraser,
    Tool::PaintBucket,
    Tool::Gradient,
    Tool::RectMarquee,
    Tool::Lasso,
    Tool::PolygonLasso,
    Tool::MagicWand,
    Tool::QuickSelection,
    Tool::ObjectSelection,
    Tool::Pen,
    Tool::Rectangle,
    Tool::VerticalTypeMask,
];

#[test]
fn pencil_only_a_finger_never_draws_fills_or_selects() {
    for tool in DRAWING_TOOLS {
        let (mut h, at) = canvas(app(PENCIL_ONLY, tool));
        let history = past_len(&h);
        contact_down(&mut h, at, None);
        assert!(h.state().0.drag.is_none(), "{tool:?}: refused at the press");
        assert!(!crate::lasso_ui::active(&h.state().0), "{tool:?}: no lasso");
        assert_eq!(past_len(&h), history, "{tool:?}: nothing committed at the press");
        // The click tools (the Magic Eraser, the wand) run on the release: refused there too.
        contact_up(&mut h, at + vec2(12.0, 0.0), None);
        assert_eq!(past_len(&h), history, "{tool:?}: nothing committed");
        assert!(h.state().0.session.active().unwrap().doc.selection.is_none(), "{tool:?}: no selection");
        // The Object Finder's own hover hint ("no pixels here") can replace the status afterwards.
        if tool != Tool::ObjectSelection {
            assert_eq!(h.state().0.ui.status, PENCIL_HINT, "{tool:?}: the hint says why");
        }
    }
}

#[test]
fn the_pencil_still_paints_at_every_pressure() {
    for force in [0.0, 0.4] {
        let (mut h, at) = canvas(app(PENCIL_ONLY, Tool::Brush));
        let base = past_len(&h);
        contact_down(&mut h, at, Some(force));
        assert!(h.state().0.drag.is_some(), "force {force}: the pencil paints");
        contact_up(&mut h, at + vec2(12.0, 0.0), Some(force));
        assert_eq!(past_len(&h), base + 1, "force {force}: one stroke");
    }
}

#[test]
fn the_pencil_still_selects_and_fills() {
    // A marquee drag (it starts on egui's drag threshold, so the proof is the selection left).
    let (mut h, at) = canvas(app(PENCIL_ONLY, Tool::RectMarquee));
    contact_down(&mut h, at, Some(0.4));
    assert_ne!(h.state().0.ui.status, PENCIL_HINT, "the marquee is not refused");
    h.input_mut().events = vec![touch(1, TouchPhase::Move, at + vec2(20.0, 5.0), Some(0.4)), egui::Event::PointerMoved(at + vec2(20.0, 5.0))];
    h.step();
    assert!(h.state().0.drag.is_some(), "the selection drag started");
    contact_up(&mut h, at + vec2(20.0, 10.0), Some(0.4));
    assert!(h.state().0.session.active().unwrap().doc.selection.is_some(), "the pencil selected");

    // A freehand lasso, which reads the raw pointer events.
    let (mut h, at) = canvas(app(PENCIL_ONLY, Tool::Lasso));
    contact_down(&mut h, at, Some(0.4));
    assert!(crate::lasso_ui::active(&h.state().0), "the pencil's lasso started");

    // A fill.
    let (mut h, at) = canvas(app(PENCIL_ONLY, Tool::PaintBucket));
    let base = past_len(&h);
    contact_down(&mut h, at, Some(0.4));
    contact_up(&mut h, at, Some(0.4));
    assert_eq!(past_len(&h), base + 1, "the pencil's tap filled");
}

#[test]
fn pencil_only_leaves_the_tools_that_are_not_drawing_to_a_finger() {
    // The Hand pans with a finger drag (it never reaches `tool_event`).
    let (mut h, at) = canvas(app(PENCIL_ONLY, Tool::Hand));
    let before = h.state().0.ui.views[0].center;
    contact_down(&mut h, at, None);
    contact_up(&mut h, at + vec2(30.0, 10.0), None);
    assert_ne!(h.state().0.ui.views[0].center, before, "the finger panned");
    assert_ne!(h.state().0.ui.status, PENCIL_HINT, "the Hand is no drawing tool");
}

#[test]
fn nobody_draws_on_a_device_without_drawing() {
    for (tool, force) in [(Tool::Brush, None), (Tool::Brush, Some(0.4)), (Tool::RectMarquee, Some(0.4)), (Tool::PaintBucket, None), (Tool::Lasso, Some(0.4))] {
        let (mut h, at) = canvas(app(DrawPolicy::None, tool));
        let history = past_len(&h);
        contact_down(&mut h, at, force);
        assert!(h.state().0.drag.is_none(), "{tool:?} {force:?}: refused at the press");
        contact_up(&mut h, at + vec2(12.0, 0.0), force);
        assert_eq!(past_len(&h), history, "{tool:?} {force:?}: nothing committed");
        assert!(h.state().0.session.active().unwrap().doc.selection.is_none(), "{tool:?}: no selection");
        assert_eq!(h.state().0.ui.status, NONE_HINT, "{tool:?} {force:?}: the hint says why");
    }
}

#[test]
fn any_input_is_the_behaviour_everywhere_else() {
    let (mut h, at) = canvas(app(DrawPolicy::AnyInput, Tool::Brush));
    contact_down(&mut h, at, None);
    assert!(h.state().0.drag.is_some(), "a finger paints when the policy is any input");
    let (mut h, at) = canvas(app(DrawPolicy::AnyInput, Tool::RectMarquee));
    contact_down(&mut h, at, None);
    assert_ne!(h.state().0.ui.status, PENCIL_HINT, "the gate takes nothing");
}

#[test]
fn a_policy_without_drawing_offers_no_drawing_tool_or_command() {
    let mut a = app(DrawPolicy::AnyInput, Tool::Brush);
    a.set_draw_policy(DrawPolicy::None);
    assert_eq!(a.ui.tool, Tool::Move, "the brush is no tool to leave selected");
    crate::tool_memory::select(&mut a, Tool::Eraser);
    assert_eq!((a.ui.tool, a.ui.status.as_str()), (Tool::Move, NONE_HINT), "and none can be selected");
    for id in ["select.all", "select.deselect", "select.inverse", "edit.fill", "edit.stroke", "filter.liquify", "layer.newFillLayer.solidColor"] {
        assert!(!crate::menus::is_enabled(&a, id), "{id}: no selection, fill or Liquify command");
    }
    for id in ["file.open", "view.rulers", "edit.undo"] {
        assert!(crate::menus::is_enabled(&a, id), "{id}: everything else is as before");
    }
    a.ui.text_edit = serde_json::from_value(json!({"layer": 0, "caret": 0, "anchor": 0, "session": "", "created": false})).unwrap();
    assert!(crate::menus::is_enabled(&a, "select.all"), "selecting all text is the Type tool's, not a selection");
    // Pencil-only offers everything.
    let p = app(PENCIL_ONLY, Tool::Brush);
    assert!(crate::menus::is_enabled(&p, "select.all") && crate::menus::is_enabled(&p, "edit.fill"));
}

#[test]
fn a_finger_joining_the_pencil_stands_nothing_down_only_under_pencil_only() {
    for (policy, survives) in [(PENCIL_ONLY, true), (DrawPolicy::AnyInput, false)] {
        let (mut h, at) = canvas(app(policy, Tool::Brush));
        contact_down(&mut h, at, Some(0.4));
        assert!(h.state().0.drag.is_some(), "{policy:?}: the pencil's stroke began");
        // The resting hand lands while the pencil draws; the shell's hook marks the pen down.
        h.state_mut().0.pen_zoom_disabled = true;
        h.input_mut().events = vec![touch(2, TouchPhase::Start, at + vec2(60.0, 40.0), None)];
        h.step();
        h.run_steps(1);
        assert_eq!(h.state().0.drag.is_some(), survives, "{policy:?}: the stroke's fate");
    }
}

/// A finger's ToolEvents never reach Liquify under `PencilOnly`; the pencil's push, freeze, thaw
/// and lasso do. Under `None` nothing does.
#[test]
fn liquify_follows_the_policy() {
    let pencil = ToolEvent::Down { x: 32.0, y: 32.0, pressure: 0.5 };
    let opened = |policy: DrawPolicy| {
        let mut app = app(policy, Tool::Move);
        // Liquify's workspace (Filter ▸ Liquify…), with pixels to push.
        app.run("select.all", json!({})).unwrap();
        app.run("edit.fill", json!({"color": "#808080"})).unwrap();
        app.run("select.deselect", json!({})).unwrap();
        crate::liquify_ui::open(&mut app, &egui::Context::default()).unwrap();
        assert!(app.distort.liquify.is_some(), "the Liquify workspace opened");
        app
    };
    let finger = |policy| {
        let mut app = opened(policy);
        app.fingers.frame(&[touch(7, TouchPhase::Start, pos2(10.0, 10.0), None), press(pos2(10.0, 10.0), true)]);
        app
    };
    for policy in [PENCIL_ONLY, DrawPolicy::None] {
        let hint = if policy == PENCIL_ONLY { PENCIL_HINT } else { NONE_HINT };
        for tool in [LiquifyTool::ForwardWarp, LiquifyTool::Freeze, LiquifyTool::Thaw, LiquifyTool::LassoMask] {
            // A finger's full gesture: nothing starts, nothing is recorded.
            let mut a = finger(policy);
            a.distort.liquify.as_mut().unwrap().opts.tool = tool;
            let before = a.distort.liquify.as_ref().unwrap().strokes.len();
            for ev in [
                ToolEvent::Down { x: 20.0, y: 20.0, pressure: 1.0 },
                ToolEvent::Move { x: 40.0, y: 20.0, pressure: 1.0 },
                ToolEvent::Up { x: 40.0, y: 20.0 },
                ToolEvent::Down { x: 20.0, y: 40.0, pressure: 1.0 },
                ToolEvent::Move { x: 40.0, y: 40.0, pressure: 1.0 },
                ToolEvent::Up { x: 40.0, y: 40.0 },
            ] {
                crate::liquify_ui::pointer(&mut a, ev, Modifiers::NONE);
            }
            assert_eq!(a.distort.liquify.as_ref().unwrap().strokes.len(), before, "{policy:?} {tool:?}: the finger recorded nothing");
            assert_eq!(a.ui.status, hint, "{policy:?} {tool:?}: the hint says why");
        }
    }
    for tool in [LiquifyTool::ForwardWarp, LiquifyTool::Freeze, LiquifyTool::Thaw, LiquifyTool::LassoMask] {
        // Under `None` the pencil is refused like the finger.
        let mut a = opened(DrawPolicy::None);
        a.distort.liquify.as_mut().unwrap().opts.tool = tool;
        crate::liquify_ui::pointer(&mut a, pencil, Modifiers::NONE);
        assert_eq!(a.ui.status, NONE_HINT, "{tool:?}: nothing pushes on a device without drawing");
        let d = a.distort.liquify.as_ref().unwrap();
        assert!(d.lasso.is_none() && d.cur.is_none(), "{tool:?}: nothing started");

        // Under `PencilOnly` the pencil's press starts the tool's own gesture.
        let mut a = opened(PENCIL_ONLY);
        a.distort.liquify.as_mut().unwrap().opts.tool = tool;
        crate::liquify_ui::pointer(&mut a, pencil, Modifiers::NONE);
        let d = a.distort.liquify.as_ref().unwrap();
        let started = match tool {
            LiquifyTool::LassoMask => d.lasso.is_some(),
            _ => d.cur.is_some(),
        };
        assert!(started, "{tool:?}: the pencil works");
    }
}

/// Select and Mask's Refine Edge Brush paints on the mask: a finger's press never does under
/// `PencilOnly`, nobody's does under `None`, the pencil's does.
#[test]
fn the_refine_edge_brush_follows_the_policy() {
    let ctx = egui::Context::default();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let strokes = |a: &PhotocraftApp| a.ui.dialogs.last().and_then(|d| d.fields.get("refineStrokes")).and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0);
    for (policy, finger, hint, paints) in [
        (PENCIL_ONLY, true, PENCIL_HINT, false),
        (PENCIL_ONLY, false, "", true),
        (DrawPolicy::None, true, NONE_HINT, false),
        (DrawPolicy::None, false, NONE_HINT, false),
        (DrawPolicy::AnyInput, true, "", true),
    ] {
        let mut a = app(policy, Tool::Move);
        a.run("select.rect", json!({"x": 0, "y": 0, "width": 32, "height": 32})).unwrap();
        let fields = json!({"__command": crate::select_mask_view::COMMAND, "__brush": true});
        a.ui.dialogs.push(crate::state::Dialog { id: 1, kind: crate::state::DialogKind::Command, fields: fields.as_object().unwrap().clone() });
        if finger {
            a.fingers.frame(&[touch(7, TouchPhase::Start, pos2(10.0, 10.0), None), press(pos2(10.0, 10.0), true)]);
        }
        let before = strokes(&a);
        a.ui.status.clear();
        assert!(crate::select_mask_view::canvas_brush(&mut a, &painter, pos2(50.0, 50.0), Some([20.0, 20.0]), 1.0), "the brush is the dialog's tool");
        assert_eq!(strokes(&a) > before, paints, "{policy:?} finger={finger}");
        assert_eq!(a.ui.status, hint, "{policy:?} finger={finger}");
    }
}

/// The workspaces whose canvas gestures paint or select take the pointer through
/// `distort_ui::pointer`, which asks the policy for them all: Select and Mask's Quick Selection
/// here. A finger's whole gesture makes no edit under `PencilOnly`, nobody's does under `None`.
#[test]
fn a_workspace_that_paints_or_selects_follows_the_policy() {
    for (policy, finger, hint, records) in [
        (PENCIL_ONLY, true, PENCIL_HINT, false),
        (PENCIL_ONLY, false, "", true),
        (DrawPolicy::None, true, NONE_HINT, false),
        (DrawPolicy::None, false, NONE_HINT, false),
        (DrawPolicy::AnyInput, true, "", true),
    ] {
        let mut a = app(policy, Tool::Move);
        // A grey layer for the Quick Selection to find, and a selection to open on.
        a.run("edit.fill", json!({"color": "#808080"})).unwrap();
        a.run("select.rect", json!({"x": 0, "y": 0, "width": 32, "height": 32})).unwrap();
        crate::menus::invoke(&mut a, &egui::Context::default(), "select.selectAndMask", json!({})).unwrap();
        if finger {
            a.fingers.frame(&[touch(7, TouchPhase::Start, pos2(10.0, 10.0), None), press(pos2(10.0, 10.0), true)]);
        }
        let edits = |a: &PhotocraftApp| a.distort.select_mask.as_ref().unwrap().describe()["edits"].as_u64().unwrap();
        let before = edits(&a);
        a.ui.status.clear();
        for ev in [ToolEvent::Down { x: 48.0, y: 48.0, pressure: 1.0 }, ToolEvent::Move { x: 56.0, y: 48.0, pressure: 1.0 }, ToolEvent::Up { x: 56.0, y: 48.0 }]
        {
            assert!(crate::distort_ui::pointer(&mut a, ev, Modifiers::NONE), "{policy:?}: the workspace takes the pointer");
        }
        assert_eq!(edits(&a) > before, records, "{policy:?} finger={finger}: an edit was recorded, or not");
        assert_eq!(a.ui.status, hint, "{policy:?} finger={finger}");
    }
}

/// A finger's click on the release frame, where the shell has just ended the finger's press for
/// the pencil and begun the pencil's own in the same frame (`apps/photocraft-ios/src/drawing.rs`):
/// the finger's click (the Paint Bucket's) is dropped with the hint, the pencil's own click fills.
#[test]
fn a_fingers_click_dropped_in_the_frame_the_pencil_takes_the_pointer() {
    let at = pos2(200.0, 200.0);
    let (mut h, _) = canvas(app(PENCIL_ONLY, Tool::PaintBucket));
    let base = past_len(&h);
    // The finger's press, on the canvas (a click tool waits for the release).
    h.input_mut().events = vec![touch(1, TouchPhase::Start, at, None), press(at, true)];
    h.step();
    h.run_steps(2);
    assert_eq!(past_len(&h), base, "nothing yet");
    // The frame the shell writes: iPadOS cancels the finger, its press ends, the pencil lands.
    h.input_mut().events = vec![
        touch(1, TouchPhase::Cancel, at, None),
        egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
        egui::Event::PointerGone,
        touch(2, TouchPhase::Start, at, Some(0.4)),
        egui::Event::PointerMoved(at),
        egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
    ];
    h.step();
    h.run_steps(2);
    assert_eq!(past_len(&h), base, "the finger's click on its release is dropped");
    assert_eq!(h.state().0.ui.status, PENCIL_HINT, "and says why");
    // The pencil lifts where it landed: its click fills.
    h.input_mut().events = vec![
        touch(2, TouchPhase::End, at, Some(0.4)),
        egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
        egui::Event::PointerGone,
    ];
    h.step();
    h.run_steps(2);
    assert_eq!(past_len(&h), base + 1, "the pencil's click, and only it");
}
