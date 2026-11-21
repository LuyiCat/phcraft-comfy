//! A finger-shaped tap and drag walk the Camera Raw Filter dialog — the flow an Android tablet
//! exercises. On Android, winit and egui-winit deliver one contact as a `Touch` event plus a
//! synthesized pointer press in the same frame (and the pair again on release), and the shell
//! runs `touch_twins::drop_twins` on the raw input before the frame; `finger_tap` replays that
//! exact shape. The dialog draws as Foreground `egui::Area`s in the embedded-viewport mode the
//! harness uses (the same branch a tablet takes), so a control that renders but never answers
//! here is the on-device "cannot adjust, cannot OK, cannot save settings" bug.

use eframe::egui::{Event, Modifiers, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

use crate::PhotocraftApp;

fn harness() -> Harness<'static, PhotocraftApp> {
    let mut h = Harness::builder().with_size(vec2(1360.0, 900.0)).with_max_steps(16).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 320, "height": 240, "background": "white"})).ok();
        app
    });
    h.run_steps(4);
    h
}

/// One Android-shaped tap: the touch and its synthesized pointer press arrive together, the
/// shell's `drop_twins` runs on the raw input (as `AndroidShell::raw_input_hook` does), then
/// the frame. Mirrors `ios_tap_flow_tests::finger_tap`.
fn finger_tap(h: &mut Harness<'static, PhotocraftApp>, at: Pos2) {
    let touch = |phase, pos| Event::Touch { device_id: TouchDeviceId(1), id: TouchId(1), phase, pos, force: None };
    let pointer = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let ctx = h.ctx.clone();
    h.input_mut().events = vec![touch(TouchPhase::Start, at), Event::PointerMoved(at), pointer(at, true)];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    h.input_mut().events = vec![touch(TouchPhase::End, at), pointer(at, false), Event::PointerGone];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    h.run_steps(2);
}

/// One Android-shaped drag through `points`: press, moves, release.
fn finger_drag(h: &mut Harness<'static, PhotocraftApp>, points: &[Pos2]) {
    let touch = |phase, pos| Event::Touch { device_id: TouchDeviceId(1), id: TouchId(1), phase, pos, force: None };
    let pointer = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let ctx = h.ctx.clone();
    let first = points[0];
    h.input_mut().events = vec![touch(TouchPhase::Start, first), Event::PointerMoved(first), pointer(first, true)];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    for &p in &points[1..] {
        h.input_mut().events = vec![touch(TouchPhase::Move, p), Event::PointerMoved(p)];
        crate::touch_twins::drop_twins(&ctx, h.input_mut());
        h.step();
    }
    let last = points[points.len() - 1];
    h.input_mut().events = vec![touch(TouchPhase::End, last), pointer(last, false), Event::PointerGone];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    h.run_steps(2);
}

/// The plain pointer form, as the desktop's mouse sees it — the control the finger shape is
/// compared against.
fn click(h: &mut Harness<'static, PhotocraftApp>, at: Pos2) {
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    h.run_steps(2);
}

fn camera_raw_open(h: &mut Harness<'static, PhotocraftApp>) {
    let ctx = h.ctx.clone();
    crate::menus::invoke(h.state_mut(), &ctx, "filter.cameraRaw", json!({})).ok();
    h.run_steps(3);
    assert!(h.state().camera_raw.is_some(), "the dialog opened");
    assert!(h.ctx.embed_viewports(), "the harness draws the dialog the embedded way a tablet does");
}

fn walk_ok(tap: fn(&mut Harness<'static, PhotocraftApp>, Pos2)) {
    let mut h = harness();
    camera_raw_open(&mut h);
    // A finger drag moves the Exposure slider: the preview's settings differ from the defaults.
    // Two carry the label (the app's panel behind the modal), so take the rightmost — the
    // dialog's settings column.
    let slider = h.query_all_by_label("Exposure").map(|n| n.rect()).max_by_key(|r| r.left().to_bits()).expect("the dialog's Exposure slider");
    let track = slider.center();
    finger_drag(&mut h, &[track, pos2(track.x + 60.0, track.y)]);
    assert_ne!(h.state().camera_raw.as_ref().unwrap().params.exposure, 0.0, "the slider answered the drag");
    // OK applies the filter and closes the dialog: one history step named for it.
    let ok = h.get_by_label("OK").rect().center();
    tap(&mut h, ok);
    assert!(h.state().camera_raw.is_none(), "OK answered the tap and closed the dialog");
    let names: Vec<String> = h.state().session.active().map(|st| st.history.entries().to_vec()).unwrap_or_default();
    assert!(names.iter().any(|n| n.contains("Camera Raw")), "{names:?}");
}

fn walk_cancel(tap: fn(&mut Harness<'static, PhotocraftApp>, Pos2)) {
    let mut h = harness();
    camera_raw_open(&mut h);
    let cancel = h.get_by_label("Cancel").rect().center();
    tap(&mut h, cancel);
    assert!(h.state().camera_raw.is_none(), "Cancel answered the tap and closed the dialog");
    let names: Vec<String> = h.state().session.active().map(|st| st.history.entries().to_vec()).unwrap_or_default();
    assert!(!names.iter().any(|n| n.contains("Camera Raw")), "{names:?}");
}

#[test]
fn a_mouse_click_oks_the_camera_raw_filter() {
    walk_ok(click);
}

/// A phone-camera-sized document (3k×5k, portrait) commits the same way: the field report
/// said the filter would not apply at that size. Slow on purpose — the develop is real.
#[test]
fn a_tap_oks_the_camera_raw_filter_on_a_3k5k_document() {
    let mut h = Harness::builder().with_size(vec2(1360.0, 900.0)).with_max_steps(24).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 3000, "height": 5000, "background": "white"})).ok();
        app
    });
    h.run_steps(4);
    camera_raw_open(&mut h);
    let ok = h.get_by_label("OK").rect().center();
    click(&mut h, ok);
    assert!(h.state().camera_raw.is_none(), "OK answered the tap and closed the dialog");
    let names: Vec<String> = h.state().session.active().map(|st| st.history.entries().to_vec()).unwrap_or_default();
    assert!(names.iter().any(|n| n.contains("Camera Raw")), "{names:?}");
}

#[test]
fn a_finger_tap_oks_the_camera_raw_filter() {
    walk_ok(finger_tap);
}

#[test]
fn a_finger_tap_cancels_the_camera_raw_filter() {
    walk_cancel(finger_tap);
}
