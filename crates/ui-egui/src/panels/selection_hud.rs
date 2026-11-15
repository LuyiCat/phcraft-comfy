//! The touch selection-mode HUD: with the Touch Interface on, a selection tool shows a small
//! floating New / Add / Subtract switch pinned above the canvas' bottom-left corner. ⇧ add and
//! ⌥ subtract need a keyboard a touch screen doesn't have; the HUD writes the same
//! `UiState::selection_mode` the options bar's buttons do, so every selection tool (and the
//! Object Selection tool's pen-only iPad gestures) can combine regions with a finger tap
//! between strokes. The desktop keeps its options bar and needs no HUD.

use crate::PhotocraftApp;

pub(crate) fn hud_id() -> egui::Id {
    egui::Id::unique("canvas-selection-mode-hud")
}

/// The HUD's height before the first frame has measured it, for placing its bottom edge above
/// the canvas' corner: three 28 pt icon buttons plus the card's inner margin.
const HEIGHT: f32 = 40.0;

/// The three modes the HUD switches. Index is `UiState::selection_mode`.
const MODES: [(&str, &str); 3] = [("square", "New selection"), ("plus", "Add to selection"), ("minus", "Subtract from selection")];

/// Show the HUD for this frame. Called once a frame from `document_area`, after the canvas.
pub fn show(app: &mut PhotocraftApp, ctx: &egui::Context) {
    // Quick Selection always adds (⌥ subtracts) and ignores the mode, so the HUD would promise
    // a New it cannot deliver; every other selection tool reads it.
    if !crate::touch::on(app)
        || !crate::tool_feedback::is_selection_tool(app.ui.tool)
        || app.ui.tool == crate::state::Tool::QuickSelection
        || app.session.active().is_none()
    {
        return;
    }
    let canvas = app.last_canvas_rect;
    if canvas.width() < 40.0 || canvas.height() < 40.0 {
        return;
    }
    // Bottom-left of the canvas work area: under the thumb of the hand holding the (pen-only)
    // selection, clear of the desktop's side toolbar and the touch layout's bottom row.
    let height = ctx.memory(|m| m.area_rect(hud_id())).map_or(HEIGHT, |r| r.height());
    let pos = egui::pos2(canvas.left() + 16.0, canvas.bottom() - height - 16.0);
    let t = crate::theme::Tokens::get(ctx);
    let frame = egui::Frame::NONE
        .fill(t.card)
        .stroke(egui::Stroke::new(1.0, t.card_border))
        .corner_radius(egui::CornerRadius::same(t.radius_lg as u8))
        .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: t.shadow })
        .inner_margin(egui::Margin::same(6));
    egui::Area::new(hud_id()).order(egui::Order::Middle).fixed_pos(pos).show(ctx, |ui| {
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, (icon, tip)) in MODES.iter().enumerate() {
                    if crate::icons::button(ui, icon, 28.0, app.ui.selection_mode == i as u8, tip).clicked() {
                        app.ui.selection_mode = i as u8;
                    }
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use serde_json::json;

    use super::*;
    use crate::state::Tool;

    /// The full app in a harness, as `marquee_tests` builds it, with the Touch Interface on or
    /// off and `tool` selected. The document is 400 × 300 at 100 % zoom.
    fn harness(tool: Tool, touch: bool) -> Harness<'static, PhotocraftApp> {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
        if touch {
            app.session.edit_prefs(|p| p.interface.touch_ui = true);
        }
        app.sync_views();
        app.ui.extras.rulers = false;
        app.ui.tool = tool;
        let mut h = Harness::builder().with_size(egui::vec2(1000.0, 700.0)).with_step_dt(0.25).build_ui_state(
            |ui, app: &mut PhotocraftApp| {
                let ctx = ui.ctx().clone();
                if !ctx.fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                    return;
                }
                crate::shortcuts::handle(app, &ctx);
                egui::CentralPanel::default().show(ui, |ui| crate::canvas::document_area(app, ui));
            },
            app,
        );
        PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
        h.run_steps(4);
        let v = &mut h.state_mut().ui.views[0];
        v.zoom = 1.0;
        v.center = [200.0, 150.0];
        v.fit_pending = false;
        h.run_steps(2);
        h
    }

    fn hud_rect(h: &Harness<'static, PhotocraftApp>) -> egui::Rect {
        h.ctx.memory(|m| m.area_rect(hud_id())).expect("the HUD is shown")
    }

    /// The HUD shows for every selection tool with the Touch Interface on, never without it and
    /// never for other tools; its buttons carry the mode they switch to.
    #[test]
    fn the_hud_shows_for_selection_tools_on_the_touch_interface_only() {
        for tool in [Tool::ObjectSelection, Tool::RectMarquee, Tool::Lasso, Tool::MagicWand] {
            let mut h = harness(tool, true);
            let rect = hud_rect(&h);
            let canvas = h.state().last_canvas_rect;
            assert!(rect.left() >= canvas.left() && rect.bottom() <= canvas.bottom() + 0.5, "pinned inside the canvas' bottom-left corner");
            assert!(h.query_by_label(tl!("Add to selection")).is_some(), "{tool:?}: the Add button is queryable");
            // The three buttons switch `selection_mode` (0 new, 1 add, 2 subtract).
            h.get_by_label(tl!("Add to selection")).click();
            h.run_steps(1);
            assert_eq!(h.state().ui.selection_mode, 1);
            h.get_by_label(tl!("Subtract from selection")).click();
            h.run_steps(1);
            assert_eq!(h.state().ui.selection_mode, 2);
            h.get_by_label(tl!("New selection")).click();
            h.run_steps(1);
            assert_eq!(h.state().ui.selection_mode, 0);
        }
        let h = harness(Tool::ObjectSelection, false);
        assert!(h.ctx.memory(|m| m.area_rect(hud_id())).is_none(), "no HUD off the Touch Interface");
        let h = harness(Tool::Brush, true);
        assert!(h.ctx.memory(|m| m.area_rect(hud_id())).is_none(), "no HUD for non-selection tools");
    }

    /// Switching the HUD to Add and dragging with the Object Selection tool combines regions
    /// without any keyboard: `select.object` runs with `"mode": "add"` and the selection
    /// outside the object survives, where New would have replaced it.
    #[test]
    fn add_mode_from_the_hud_combines_object_selection_regions() {
        let mut h = harness(Tool::ObjectSelection, true);
        h.state_mut().ui.extras.snap = false;
        h.state_mut().run("select.rect", json!({"x": 10, "y": 10, "width": 20, "height": 20})).unwrap();
        h.state_mut().run("paint.pencil", json!({"points": [[220, 150]], "size": 60, "color": "#ff0000"})).unwrap();
        h.get_by_label(tl!("Add to selection")).click();
        h.run_steps(1);
        assert_eq!(h.state().ui.selection_mode, 1);
        let view = crate::canvas::ViewXform {
            rect: crate::rulers::content_rect(h.state(), h.state().last_canvas_rect),
            zoom: h.state().ui.views[0].zoom,
            center: h.state().ui.views[0].center,
            flip: h.state().ui.view.flip_horizontal,
            rotation: h.state().ui.views[0].rotation,
            aspect: h.state().ui.view.display_aspect(),
        };
        let at = |x: f32, y: f32| view.to_screen(x, y);
        for (p, pressed) in [(at(150.0, 100.0), true), (at(290.0, 200.0), false)] {
            h.event(egui::Event::PointerMoved(p));
            h.run_steps(1);
            h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            h.run_steps(1);
        }
        let app = h.state();
        assert!(!app.ui.status_error, "{}", app.ui.status);
        let (command, params) = app.session.journal.last().expect("the drag dispatched a command");
        assert_eq!(command, "select.object");
        assert_eq!(params["mode"], "add", "the HUD's mode, not a keyboard modifier");
        let sel = app.session.active().unwrap().doc.selection.as_ref().expect("a selection exists");
        assert!(sel.sample_channel(20, 20, 0) > 0.99, "the earlier rectangle survives outside the object");
    }
}
