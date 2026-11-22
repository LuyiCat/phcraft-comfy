//! A finger-shaped tap must click a button inside a scrolling area: the Android file
//! browser's rows are, and a report from the device named them dead. The finger shape is the
//! one egui-winit synthesizes (touch plus emulated pointer, deduped by `drop_twins`), and the
//! ScrollArea drags on touch screens — both together must still leave a tap a click.
use egui::{Event, Modifiers, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn finger_tap(h: &mut Harness<'static, u32>, at: Pos2) {
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

#[test]
fn a_finger_tap_clicks_a_button_inside_a_scrolling_area() {
    let clicked = 0u32;
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_max_steps(16).build_ui_state(
        |ui, state: &mut u32| {
            egui::ScrollArea::vertical().id_salt("scroller").auto_shrink([false, false]).show(ui, |ui| {
                ui.set_min_height(600.0); // overflow the 300pt viewport
                if ui.button("Tap me").clicked() {
                    *state += 1;
                }
                ui.label("filler");
            });
        },
        clicked,
    );
    h.run_steps(3);
    let button = h.get_by_label("Tap me").rect().center();
    finger_tap(&mut h, button);
    assert_eq!(*h.state(), 1, "the button inside the ScrollArea answered the finger tap");
}
