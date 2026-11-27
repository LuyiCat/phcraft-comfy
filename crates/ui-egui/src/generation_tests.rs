//! Image generation in the interface (`PhotocraftApp::generation`, generation.rs): a shell whose
//! build leaves the transport out switches it off, and nothing then names generation — no
//! Generate button in the portrait drawer, no File or Window menu item, nothing in the list that
//! feeds the palette and the shortcut editor — and `menus::invoke` refuses the ids. The default
//! keeps every entry, for every other shell.

use serde_json::json;

use crate::PhotocraftApp;
use crate::dock::Group;
use crate::generation::Generation;

fn fresh() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default())
}

/// The ids in the menu list that are generation commands.
fn generation_ids(app: &PhotocraftApp) -> Vec<String> {
    crate::menus::menu_items(app).into_iter().map(|i| i.id).filter(|id| Generation::owns(id)).collect()
}

/// Every other id, in order.
fn other_ids(app: &PhotocraftApp) -> Vec<String> {
    crate::menus::menu_items(app).into_iter().map(|i| i.id).filter(|id| !Generation::owns(id)).collect()
}

/// The drawer's groups, top to bottom.
fn drawer_groups(app: &PhotocraftApp) -> Vec<Group> {
    crate::panels::drawer_entries(app).into_iter().map(|(group, _, _)| group).collect()
}

#[test]
fn the_default_lists_every_generation_entry() {
    let app = fresh();
    assert_eq!(app.generation, Generation::Offered);
    let ids = generation_ids(&app);
    for id in ["window.panel.comfy", "ai.comfyui.generate", "ai.comfyui.fill"] {
        assert!(ids.iter().any(|i| i == id), "{id} is listed");
    }
    assert_eq!(drawer_groups(&app).len(), 8);
    assert!(drawer_groups(&app).contains(&Group::Comfy));
}

#[test]
fn a_build_without_generation_lists_no_generation_item_and_nothing_else_changes() {
    let mut app = fresh();
    let other = other_ids(&app);
    let listed = generation_ids(&app).len();
    let total = crate::menus::menu_items(&app).len();
    assert!(listed >= 3, "the default lists the generation items: {listed}");
    app.set_generation_removed();
    assert_eq!(app.generation, Generation::Removed);
    assert!(generation_ids(&app).is_empty());
    assert_eq!(crate::menus::menu_items(&app).len(), total - listed, "only the generation items went");
    assert_eq!(other_ids(&app), other, "everything else stays, in order");
}

#[test]
fn the_portrait_drawer_has_no_generate_button_without_generation() {
    let mut app = fresh();
    let offered = drawer_groups(&app);
    app.set_generation_removed();
    let removed = drawer_groups(&app);
    assert_eq!(removed.len(), 7);
    assert!(!removed.contains(&Group::Comfy));
    assert_eq!(removed, offered.into_iter().filter(|g| *g != Group::Comfy).collect::<Vec<_>>());
}

#[test]
fn switching_it_off_closes_the_panel_and_the_drawers_own_tab() {
    let mut app = fresh();
    let ctx = egui::Context::default();
    // With generation offered, the Window menu's id opens the panel.
    crate::menus::invoke(&mut app, &ctx, "window.panel.comfy", json!({})).expect("the Window menu's id opens the panel");
    assert!(app.ui.panels.comfy);
    app.touch_drawer = Some(Group::Comfy);
    app.set_generation_removed();
    assert!(!app.ui.panels.comfy);
    assert_eq!(app.touch_drawer, None);

    // Another group's drawer stays open.
    let mut app = fresh();
    app.touch_drawer = Some(Group::Layers);
    app.set_generation_removed();
    assert_eq!(app.touch_drawer, Some(Group::Layers));
}

#[test]
fn no_source_runs_a_generation_command_without_generation() {
    let ctx = egui::Context::default();
    let mut app = fresh();
    app.set_generation_removed();
    for id in ["window.panel.comfy", "ai.comfyui.fill", "ai.comfyui.generate", "ai.comfyui.workflows.list"] {
        let why = crate::menus::invoke(&mut app, &ctx, id, json!({})).expect_err(&format!("{id} is refused"));
        assert!(why.contains("not part of this build"), "{id}: {why}");
    }
    assert!(!app.ui.panels.comfy, "the panel stayed closed");

    // A command that is no generation still runs.
    assert!(crate::menus::invoke(&mut app, &ctx, "edit.search", json!({})).is_ok());
}
