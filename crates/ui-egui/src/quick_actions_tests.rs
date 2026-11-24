//! The touch quick-actions column (panels/quick_actions.rs): Undo, Redo, Copy, Cut, Paste at
//! the canvas's top-right corner, left of the end dock — taps only, dispatching the engine's
//! own edit commands.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use serde_json::json;

use crate::PhotocraftApp;

/// The window these tests are written for.
const WINDOW: egui::Vec2 = egui::vec2(1200.0, 800.0);

/// Run `count` frames of `h`, each with the window restated at [`WINDOW`].
///
/// A frame takes the harness's screen rect and leaves none behind, so every later frame
/// inherits the one before; and the frame after a zoom change carries that rect scaled by the
/// zoom ratio (upstream egui keeps a zoomed window's coordinates in place that way). The Touch
/// Interface's magnification, ×1.4 at this window size, is a zoom change on the first frame, so
/// once the size is forgotten the window stays 857×571 where egui's checkout is unpatched (CI)
/// and 1200×800 where `apps/photocraft-ios/patches` has been applied to it (the iOS build
/// script does that; a developer machine). Restated before every frame, the window is [`WINDOW`]
/// again on both from the next frame on — and where this column sits is what these tests measure.
fn frames(h: &mut Harness<'_, PhotocraftApp>, count: usize) {
    for _ in 0..count {
        h.set_size(WINDOW);
        h.step();
    }
}

fn touch_harness() -> Harness<'static, PhotocraftApp> {
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(WINDOW).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(s, crate::Services::default());
        app.session.edit_prefs(|p| {
            p.interface.touch_ui = true;
            // The label-based assertions need English whatever a parallel i18n live test did
            // to the global language (the frame re-reads this pref every pass).
            p.interface.language = "en".into();
        });
        app
    });
    frames(&mut h, 4);
    h
}

fn layer_names(h: &Harness<'_, PhotocraftApp>) -> Vec<String> {
    h.state().session.active().unwrap().doc.layers.iter().map(|l| l.name.clone()).collect()
}

/// The column's button labelled `label`, the one inside the column's card (the Edit menu carries
/// the same names).
fn column_button<'a>(h: &'a Harness<'_, PhotocraftApp>, label: &'a str) -> Option<Node<'a>> {
    let card = crate::panels::quick_actions::rect(&h.ctx).expect("the column drew");
    h.query_all_by_label(label).into_iter().find(|n| card.contains(n.rect().center()))
}

#[test]
fn the_column_sits_at_the_canvas_top_right_and_only_with_the_touch_interface() {
    let h = touch_harness();
    let card = crate::panels::quick_actions::rect(&h.ctx).expect("the column drew");
    let canvas = h.state().last_canvas_rect;
    let top_right = egui::pos2(canvas.right(), canvas.top());
    assert!(card.right() <= canvas.right() && card.left() >= canvas.right() - 80.0, "the column hugs the canvas's right edge");
    assert!(card.top() >= canvas.top() && card.top() <= canvas.top() + 40.0, "and its top");
    assert!(card.right() < top_right.x - 4.0, "inset from the corner");
    for label in ["Undo", "Redo", "Copy", "Cut", "Paste"] {
        assert!(column_button(&h, label).is_some(), "{label} is in the column");
    }
    // Chrome, not a window: the column is on the background layer order, where no tap lifts it
    // above the floating editor windows, menus and dialogs — those cover it.
    let layer = egui::LayerId::new(egui::Order::Background, egui::Id::unique("quick-actions"));
    assert_eq!(h.ctx.layer_id_at(card.center()), Some(layer), "the column sits on the background layer order");
    // Without the Touch Interface there is no column (the desktop keeps its shortcuts).
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(WINDOW).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        PhotocraftApp::new(s, crate::Services::default())
    });
    frames(&mut h, 4);
    assert!(crate::panels::quick_actions::rect(&h.ctx).is_none(), "no column in the pointer-first layout");
}

/// A pointer click at `at`, as the touch harness's taps are shaped.
fn tap(h: &mut Harness<'_, PhotocraftApp>, at: egui::Pos2) {
    for pressed in [true, false] {
        h.input_mut().events = vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }];
        frames(h, 1);
    }
    frames(h, 3);
}

/// Chrome, not a window: a floating editor window over the column covers its buttons and stays
/// above them however the column is tapped (an area is brought to the front only within its own
/// layer order); the taps that land on the covered part reach the window.
#[test]
fn a_floating_window_over_the_column_covers_it_however_the_column_is_tapped() {
    let mut h = touch_harness();
    h.state_mut().session.execute("layer.new.layer", json!({"name": "extra"})).unwrap();
    h.state_mut().ui.panels.brush_settings = true;
    frames(&mut h, 4);
    let window_id = egui::Id::unique("brush-settings");
    let window_layer = egui::LayerId::new(egui::Order::Middle, window_id);
    let column_layer = egui::LayerId::new(egui::Order::Background, egui::Id::unique("quick-actions"));
    let covers = |h: &Harness<'_, PhotocraftApp>, at| h.ctx.layer_id_at(at) == Some(window_layer);
    let undo = column_button(&h, "Undo").expect("Undo").rect();
    // Brush Settings opens wherever the canvas's top-right corner puts it (brush_panel::window):
    // beside the column, over part of it, over all of it. It is parked here instead, its right
    // edge through the middle of the column, so the buttons show beside it whatever the layout.
    let mut state = egui::AreaState::load(&h.ctx, window_id).expect("Brush Settings floats");
    let width = state.size.map_or(0.0, |s| s.x);
    state.set_left_top_pos(egui::pos2(undo.center().x - width, undo.top() - 20.0));
    h.ctx.memory_mut(|m| m.areas_mut().set_state(window_layer, state));
    frames(&mut h, 3);
    let window = h.ctx.memory(|m| m.area_rect(window_id)).expect("Brush Settings floats");
    let screen = h.ctx.content_rect();
    // A quarter across a button is under the window; three quarters is the part still showing.
    let across = |button: egui::Rect, t: f32| egui::pos2(button.left() + t * button.width(), button.center().y);
    let (covered, sliver) = (across(undo, 0.25), across(undo, 0.75));
    assert!(covers(&h, covered), "the window lies over the column's buttons: {window:?} vs {undo:?} in {screen:?}");
    assert_eq!(h.ctx.layer_id_at(sliver), Some(column_layer), "the rest of Undo shows beside the window: {window:?} vs {undo:?} in {screen:?}");
    // The covered part takes no tap.
    tap(&mut h, covered);
    assert_eq!(layer_names(&h), ["Background", "extra"], "the covered part of Undo took no tap");
    // The part showing answers a tap, which brings the column to the front of its own order —
    // under the window still.
    tap(&mut h, sliver);
    assert_eq!(layer_names(&h), ["Background"], "the visible part of Undo took the tap");
    let redo = column_button(&h, "Redo").expect("Redo").rect();
    assert!(h.state().session.is_enabled("edit.redo"), "so a tap on that Redo would redo");
    let redo_covered = across(redo, 0.25);
    assert!(covers(&h, redo_covered), "the window still covers the column after taps on the column");
    tap(&mut h, redo_covered);
    assert_eq!(layer_names(&h), ["Background"], "the covered Redo took no tap");
    assert!(covers(&h, redo_covered), "and the window is still there over it");
}

#[test]
fn a_tap_on_undo_and_redo_round_trips_a_new_layer() {
    let mut h = touch_harness();
    h.state_mut().session.execute("layer.new.layer", json!({"name": "extra"})).unwrap();
    frames(&mut h, 2);
    assert_eq!(layer_names(&h), ["Background", "extra"]);
    column_button(&h, "Undo").expect("Undo").click();
    frames(&mut h, 3);
    assert_eq!(layer_names(&h), ["Background"], "the column's Undo undid the new layer");
    column_button(&h, "Redo").expect("Redo").click();
    frames(&mut h, 3);
    assert_eq!(layer_names(&h), ["Background", "extra"], "and Redo brought it back");
}

/// Copy then Paste through the column: the selection's pixels land as a new layer, with the
/// greyed-out Paste enabled only after the copy — the Edit menu's own enablement.
/// Copy then Paste through the column: the active layer's pixels land as a new layer, with
/// Paste greyed until the copy — the Edit menu's own enablement. (The layer copied is the
/// white Background: a fresh empty layer holds no pixels to copy.)
#[test]
fn copy_then_paste_through_the_column_lands_a_new_layer() {
    let mut h = touch_harness();
    h.state_mut().session.execute("layer.new.layer", json!({"name": "paint"})).unwrap();
    h.state_mut().session.execute("layer.select", json!({"name": "Background"})).unwrap();
    frames(&mut h, 2);
    assert!(!h.state().session.is_enabled("edit.paste"), "Paste is greyed with an empty clipboard");
    column_button(&h, "Copy").expect("Copy").click();
    frames(&mut h, 3);
    assert!(h.state().session.clipboard.is_some(), "the copy filled the session clipboard");
    assert!(h.state().session.is_enabled("edit.paste"), "Paste is enabled after the copy");
    column_button(&h, "Paste").expect("Paste").click();
    frames(&mut h, 3);
    assert_eq!(layer_names(&h).len(), 3, "the paste landed the copy as a new layer");
    // And the column's own Undo takes it back off.
    column_button(&h, "Undo").expect("Undo").click();
    frames(&mut h, 3);
    assert_eq!(layer_names(&h).len(), 2, "Undo removed the pasted layer");
}
