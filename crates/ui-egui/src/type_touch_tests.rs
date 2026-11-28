//! The Type tool under a finger: a tap on the empty canvas starts a new text layer's edit, as a
//! mouse click does, and the edit stays open once the finger lifted and the pointer is gone.

use egui::{Pos2, TouchDeviceId, TouchId, TouchPhase, pos2};
use egui_kittest::Harness;

use crate::PhotocraftApp;
use crate::canvas::document_area;
use crate::draw_policy::DrawPolicy;
use crate::state::Tool;

fn app(policy: DrawPolicy, tool: Tool) -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
    app.run("file.new", serde_json::json!({"width": 64, "height": 64, "background": "transparent"})).unwrap_or_else(|e| panic!("file.new: {e}"));
    app.sync_views();
    app.set_draw_policy(policy);
    app.ui.tool = tool;
    app
}

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

fn touch(phase: TouchPhase, pos: Pos2) -> egui::Event {
    egui::Event::Touch { device_id: TouchDeviceId(0), id: TouchId(1), phase, pos, force: None }
}
fn press(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

/// The event stream egui-winit writes for a finger tap: the contact and the press, then the lift,
/// then the pointer leaving.
fn finger_tap(h: &mut Harness<'_, (PhotocraftApp, bool)>, at: Pos2) {
    h.input_mut().events = vec![touch(TouchPhase::Start, at), press(at, true)];
    h.step();
    h.input_mut().events = vec![touch(TouchPhase::End, at), press(at, false)];
    h.step();
    h.input_mut().events = vec![egui::Event::PointerGone];
    h.step();
}

#[test]
fn a_fingers_tap_with_the_type_tool_starts_the_new_layers_edit() {
    // The iPad's policy: the Type tool is no drawing tool, so a finger may use it.
    let (mut h, at) = canvas(app(DrawPolicy::PencilOnly, Tool::Type));
    finger_tap(&mut h, at);
    h.run_steps(4);
    let app = &h.state().0;
    let layer = app.session.active().expect("document").active_layer.expect("the tap made a layer");
    let edit = app.ui.text_edit.as_ref().expect("the new layer's edit is open");
    assert_eq!(edit.layer, layer.0, "and edits that layer");
    assert!(edit.created, "as a new layer's edit");
}

#[test]
fn a_fingers_tap_with_the_type_tool_on_a_device_without_drawing_still_starts_the_edit() {
    // The iPhone offers the Type tool too (`Tool::is_drawing` leaves it out): typing is not drawing.
    let (mut h, at) = canvas(app(DrawPolicy::None, Tool::Type));
    finger_tap(&mut h, at);
    h.run_steps(4);
    assert!(h.state().0.ui.text_edit.is_some(), "the new layer's edit is open");
}
