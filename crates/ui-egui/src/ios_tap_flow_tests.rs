//! A finger-shaped tap walks File › New › Create in the portrait Touch Interface layout — the
//! flow a real iPad exercises ("can't create a file"). On iOS, winit and egui-winit deliver one
//! contact as a `Touch` event plus a synthesized pointer press in the same frame (and the pair
//! again on release), and the iOS shell runs `touch_twins::drop_twins` on the raw input before
//! the frame — this test replays exactly that shape through the whole app, in the portrait touch
//! chrome, where the menu bar, the menu popup, the New Document dialog and its Create button
//! all have to answer a tap.

use eframe::egui::{Event, Modifiers, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::PhotocraftApp;

fn harness() -> Harness<'static, PhotocraftApp> {
    // pixels_per_point 2 as on the iPad: the Touch Interface zoom then leaves ~760 pt of
    // design width, less than the New Document dialog's 800 pt design minimum.
    let mut h = Harness::builder().with_size(vec2(834.0, 1194.0)).with_pixels_per_point(2.0).with_max_steps(16).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.session.edit_prefs(|p| p.interface.touch_ui = true);
        app
    });
    h.run_steps(4);
    h
}

/// One iOS-shaped tap: the touch and its synthesized pointer press arrive together, the shell's
/// `drop_twins` runs on the raw input (as `IosShell::raw_input_hook` does), then the frame.
fn finger_tap(h: &mut Harness<'static, PhotocraftApp>, at: Pos2) {
    let touch = |phase, pos| Event::Touch { device_id: TouchDeviceId(1), id: TouchId(1), phase, pos, force: None };
    let pointer = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    let ctx = h.ctx.clone();
    h.input_mut().events = vec![touch(TouchPhase::Start, at), pointer(at, true)];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    h.input_mut().events = vec![touch(TouchPhase::End, at), pointer(at, false), Event::PointerGone];
    crate::touch_twins::drop_twins(&ctx, h.input_mut());
    h.step();
    h.run_steps(2);
}

/// The same walk with plain pointer clicks, as the desktop and the simulator's mouse see it —
/// the control the finger shape is compared against.
fn click(h: &mut Harness<'static, PhotocraftApp>, at: Pos2) {
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    h.run_steps(2);
}

fn walk_new_document(h: &mut Harness<'static, PhotocraftApp>, tap: fn(&mut Harness<'static, PhotocraftApp>, Pos2)) {
    // Home is up (no documents, Auto Show on): its rail carries the New file button.
    assert!(h.state().session.documents().is_empty());
    assert!(h.state_mut().home_screen_up(), "the Home screen is up");

    // File › New… opens the New Document dialog.
    let file = h.get_by_label("File").rect().center();
    tap(h, file);
    let new_item = h.query_by_label_contains("New…").expect("the File menu opened and shows New…").rect().center();
    tap(h, new_item);
    assert_eq!(h.state().ui.dialogs.len(), 1, "the New Document dialog opened");

    // Create makes a document. The button must be on the screen: an 11-inch portrait at the
    // Touch Interface scale has less room than the dialog's 800 pt design minimum, which used
    // to push it past the right edge (invisible, untappable — "can't create a file").
    let create_rect = h.get_by_label("Create").rect();
    let screen = h.ctx.input(|i| i.content_rect());
    assert!(
        create_rect.right() <= screen.right() && create_rect.left() >= screen.left() && create_rect.bottom() <= screen.bottom(),
        "Create fully on the screen: {create_rect:?} vs {screen:?}"
    );
    let create = create_rect.center();
    tap(h, create);
    assert_eq!(h.state().session.documents().len(), 1, "Create opened a document");
}

#[test]
fn a_mouse_click_creates_a_document_from_the_menu() {
    let mut h = harness();
    walk_new_document(&mut h, click);
}

#[test]
fn a_finger_tap_creates_a_document_from_the_menu() {
    let mut h = harness();
    walk_new_document(&mut h, finger_tap);
}

/// The portrait chrome's geometry (the desktop layout's stand-ins): the tool strip — the
/// desktop toolbar's row — sits under the options bar, above the canvas, and the panel
/// drawer sits under the canvas. Regression: the strip used to be a bottom panel between
/// the drawer and the status bar.
#[test]
fn portrait_tool_strip_above_the_canvas_and_the_drawer_below() {
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", serde_json::json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(vec2(834.0, 1194.0)).with_pixels_per_point(2.0).with_max_steps(16).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(s, crate::Services::default());
        app.session.edit_prefs(|p| {
            p.interface.touch_ui = true;
            p.interface.language = "en".into();
        });
        app
    });
    h.run_steps(6);
    let canvas = h.state().last_canvas_rect;
    assert!(canvas.is_positive(), "a document is open and laid out: {canvas:?}");
    // The Move Tool button shows in the options bar and in the portrait strip; both are above
    // the canvas, the strip one right at its top edge.
    let moves: Vec<_> = h.query_all_by_label("Move Tool").map(|n| n.rect()).collect();
    assert!(moves.iter().all(|r| r.bottom() <= canvas.top() + 1.0), "every Move Tool button is above the canvas: {moves:?} vs {canvas:?}");
    assert!(
        moves.iter().any(|r| r.top() >= canvas.top() - 90.0 && r.bottom() <= canvas.top() + 1.0),
        "the portrait tool strip sits right above the canvas: {moves:?} vs {canvas:?}"
    );
    // The drawer's group buttons sit right below the canvas.
    let drawer: Vec<_> = h.query_all_by_label("Layers").map(|n| n.rect()).filter(|r| r.top() >= canvas.bottom() - 1.0).collect();
    assert!(!drawer.is_empty(), "the Layers drawer group button sits below the canvas {canvas:?}");
}

/// The portrait chrome must settle: with the drawer open (the Layers group, as the Android
/// shell starts), the canvas geometry and the interface zoom stay identical frame to frame.
/// The zoom/geometry feedback — "the page keeps zooming in and out by itself" — is the
/// flicker this guards.
#[test]
fn portrait_geometry_and_zoom_settle_with_the_drawer_open() {
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", serde_json::json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(vec2(834.0, 1194.0)).with_pixels_per_point(2.0).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(s, crate::Services::default());
        app.session.edit_prefs(|p| {
            p.interface.touch_ui = true;
            p.interface.language = "en".into();
        });
        app
    });
    h.run_steps(4);
    let layers_button = h.get_by_label("Layers").rect().center();
    finger_tap(&mut h, layers_button);
    let mut rects = Vec::new();
    let mut zooms = Vec::new();
    for _ in 0..4 {
        h.step();
        h.step();
        rects.push(h.state().last_canvas_rect);
        zooms.push(h.ctx.zoom_factor());
    }
    assert!(!crate::layer_row_ui::recorded(&h.ctx).is_empty(), "the bottom drawer displays the Layers panel's rows");
    assert!(rects.windows(2).all(|w| w[0] == w[1]), "the canvas rect must not move between frames: {rects:?}");
    assert!(zooms.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-4), "the interface zoom must settle: {zooms:?}");
}

#[test]
fn portrait_tool_flyout_stays_open_after_a_context_tap() {
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", serde_json::json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(vec2(834.0, 1194.0)).with_pixels_per_point(2.0).with_max_steps(16).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(s, crate::Services::default());
        app.session.edit_prefs(|p| {
            p.interface.touch_ui = true;
            p.interface.language = "en".into();
        });
        app
    });
    h.run_steps(6);
    h.get_by_label("Brush Tool").click_secondary();
    h.run_steps(2);
    let id = egui::Id::unique("portrait-tool-flyout");
    assert!(h.ctx.data(|d| d.get_temp::<(egui::Id, egui::Rect)>(id)).is_some(), "the context tap opens the tool choices");
    h.run_steps(4);
    assert!(h.ctx.data(|d| d.get_temp::<(egui::Id, egui::Rect)>(id)).is_some(), "the choices do not flash away on release");
    assert!(h.query_by_label("Pencil Tool").is_some(), "the flyout's alternative tool is visible");
}

#[test]
fn iphone_portrait_opens_the_layers_drawer_with_its_rows_on_screen() {
    let mut s = photocraft_engine::Session::new();
    s.execute("file.new", serde_json::json!({"width": 64, "height": 48})).unwrap();
    let mut h = Harness::builder().with_size(vec2(393.0, 852.0)).with_pixels_per_point(3.0).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(s, crate::Services::default());
        app.session.edit_prefs(|p| {
            p.interface.touch_ui = true;
            p.interface.language = "en".into();
        });
        app
    });
    h.run_steps(6);
    let layers = h.get_by_label("Layers").rect();
    let screen = h.ctx.input(|i| i.content_rect());
    assert!(screen.contains_rect(layers), "the Layers entry fits the iPhone screen: {layers:?} vs {screen:?}");
    finger_tap(&mut h, layers.center());
    let rows = crate::layer_row_ui::recorded(&h.ctx);
    assert!(!rows.is_empty(), "the iPhone drawer shows the document's layers");
    assert!(rows.iter().all(|row| screen.intersects(row.row)), "the rows are on screen: {:?}", rows.iter().map(|row| row.row).collect::<Vec<_>>());
}
