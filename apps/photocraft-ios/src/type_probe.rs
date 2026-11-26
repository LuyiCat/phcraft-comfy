//! Testing aid for the Type tool on a simulator or device with no finger. Development builds
//! only: App Store uploads leave out the default `aigc` feature, and `build.rs` compiles the
//! other half (`ios/PCTextInput.m`, `PC_TYPE_PROBE`) only beside it.
//!
//! * `PHOTOCRAFT_TYPE_PROBE`: a new document, a Type edit at its centre, then a pinyin keyboard
//!   session replayed through the same `UITextInput` methods UIKit's keyboard calls — marked
//!   text, commits, backspaces — one action every [`STEP_FRAMES`]. The layer's text is read back
//!   after each action, so the launch trail names the action a dropped or mangled composition.
//! * `PHOTOCRAFT_FONT_SPECIMEN`: a white document with one line of Chinese per embedded font
//!   family of a CJK script, so a screenshot shows every bundled face drawing.

use egui::Modifiers;
use photocraft_doc::LayerContent;
use photocraft_ui_egui::{PhotocraftApp, Tool, type_tool};
use serde_json::json;

unsafe extern "C" {
    safe fn pc_type_probe_step_count() -> isize;
    safe fn pc_type_probe_step(step: isize) -> bool;
}

/// The frame the edit starts on: past the first frames, with the font menus ready.
const START_FRAME: u32 = 30;
/// Frames between keyboard actions, so each one is applied and drawn before the next.
const STEP_FRAMES: u32 = 4;

pub(super) fn frame(shell: &mut super::IosShell, ctx: &egui::Context) {
    // An idle home screen draws no further frame on its own; keep them coming until the session ends.
    ctx.request_repaint();
    if shell.frames == START_FRAME {
        start_edit(&mut shell.app);
        super::breadcrumb(&format!("type probe: editing {:?}", edited_text(&shell.app)));
        ctx.request_repaint();
        return;
    }
    let Some(elapsed) = shell.frames.checked_sub(START_FRAME + STEP_FRAMES) else { return };
    if elapsed % STEP_FRAMES != 0 {
        return;
    }
    let step = isize::try_from(elapsed / STEP_FRAMES).unwrap_or(isize::MAX);
    if step >= pc_type_probe_step_count() {
        return;
    }
    // The action queues its events for the next pass; report the text the previous one left.
    super::breadcrumb(&format!("type probe before step {step}: {:?}", edited_text(&shell.app)));
    if !pc_type_probe_step(step) {
        super::breadcrumb("type probe: no first responder taking text input");
    }
    ctx.request_repaint();
}

/// The text of the layer being edited.
fn edited_text(app: &PhotocraftApp) -> Option<String> {
    let ed = app.ui.text_edit.as_ref()?;
    match &app.session.active()?.doc.layer(photocraft_doc::LayerId(ed.layer))?.content {
        LayerContent::Text(t) => Some(t.text.clone()),
        _ => None,
    }
}

/// The Type tool's click on an empty canvas, which starts a new point text layer.
fn start_edit(app: &mut PhotocraftApp) {
    if app.session.active().is_none()
        && let Err(e) = app.run("file.new", json!({"width": 1024, "height": 768, "background": "white"}))
    {
        log::warn!("type probe file.new: {e}");
        return;
    }
    app.ui.tool = Tool::Type;
    let (w, h) = app.session.active().map_or((1024.0, 768.0), |st| (f64::from(st.doc.size.width), f64::from(st.doc.size.height)));
    let at = [w / 2.0, h / 2.0];
    type_tool::pointer_down(app, at[0], at[1], Modifiers::NONE);
    type_tool::pointer_up(app, at, at);
}

/// `PHOTOCRAFT_FONT_SPECIMEN`: one line of Chinese per embedded CJK font family.
pub(super) fn font_specimen(app: &mut PhotocraftApp) {
    let mut families: Vec<&str> = Vec::new();
    for f in photocraft_text::CRAFT_FONTS {
        if f.scripts.iter().any(|s| matches!(*s, "Hans" | "Hant" | "Jpan" | "Kore")) && !families.contains(&f.family) {
            families.push(f.family);
        }
    }
    let rows = u32::try_from(families.len()).unwrap_or(0);
    if app.run("file.new", json!({"width": 1700, "height": 90 + rows * 78, "background": "white"})).is_err() {
        return;
    }
    for (row, family) in families.iter().enumerate() {
        let y = 96 + row as u32 * 78;
        let _ = app.run(
            "type.create",
            json!({"x": 40, "y": y, "text": format!("中文字体 永东国爱龟 龙凤 龜 點 「{family}」"), "font": family, "size": 34, "color": "#000000"}),
        );
    }
    super::breadcrumb(&format!("font specimen: {} CJK families: {}", families.len(), families.join(", ")));
}
