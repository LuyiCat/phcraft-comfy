//! The iPad shell: the same ui-egui app on Metal. winit's iOS backend delivers touches as egui
//! input (Apple Pencil force included), the canvas and tools work unchanged; platform services
//! the OS doesn't give an iPad app (desktop file dialogs, the clipboard bridge, crash recovery)
//! stay unset and the shell degrades gracefully, as in the browser. iOS alone decides who may
//! draw, fill and select (drawing.rs): the Apple Pencil alone on the iPad, nobody on the iPhone.

use std::path::{Path, PathBuf};

use photocraft_codecs::{ChannelLayout, EncodeOptions, Image};
use photocraft_doc::Document;
use photocraft_ui_egui::theme::ThemeKind;

// The last tap's received position, painted as a crosshair under `PHOTOCRAFT_DEBUG_IOS`.
thread_local! {
    static LAST_TAP: std::cell::RefCell<Option<egui::Pos2>> = const { std::cell::RefCell::new(None) };
}
#[cfg(target_os = "ios")]
use objc2_ui_kit::UICoordinateSpace;
use photocraft_ui_egui::{FileDialogAnswer, FileDialogRequest, FileStamp, PhotocraftApp, Services};

// Unconditional: the services() below references it on every host (clippy, tests); the
// extern "C" pc_* symbols only link on iOS, where the ObjC file is compiled in.
mod clipboard;
mod drawing;
#[cfg(target_os = "ios")]
mod ime;
mod long_press;
mod picker;
// Development builds only (the `aigc` feature App Store uploads leave out): replays a keyboard
// session and draws a font specimen where no finger can reach, and replays finger and pencil
// touches on the canvas.
#[cfg(all(target_os = "ios", feature = "aigc"))]
mod draw_probe;
#[cfg(all(target_os = "ios", feature = "aigc"))]
mod type_probe;
// Feature-gated with the Image API transport: App Store Review builds pass
// `--no-default-features`, which drops the optional `ureq` dependency too.
#[cfg(feature = "aigc")]
mod api_gen;

fn main() -> eframe::Result {
    // Route records to stderr (`simctl launch --console-pty`): eframe quits with exit(0) on a
    // startup error, and without a logger that error is dropped silently.
    let _ = log::set_logger(&StderrLogger).map(|()| log::set_max_level(log::LevelFilter::Info));
    breadcrumb("launched");
    discard_saved_scene_state();
    install_locale();
    install_panic_log();
    picker::install();
    let result = eframe::run_native(
        "PhotoComfy",
        native_options(),
        Box::new(|cc| {
            breadcrumb("app created");
            Ok(Box::new(IosShell::new(cc)))
        }),
    );
    breadcrumb(&format!("run_native returned: {result:?}"));
    result
}

/// Startup trail, for a device where stderr is invisible: each step appends to
/// `Documents/launch.log` (Finder file sharing). A white-screen crash names the step that never
/// happened — or, with no entries at all, that the process never ran our binary at all
/// (signing/build), since the trail's first line is written before anything else.
fn breadcrumb(step: &str) {
    let line = format!("[{:?}] {step}\n", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    log::info!("{step}");
    if let Some(dir) = documents_dir() {
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("launch.log")) {
            let _ = write!(file, "{line}");
        }
    }
}

/// winit recreates its window from scratch every launch (classic lifecycle), but once the
/// Info.plist adopts scenes, UIKit persists the scene session in `Library/Saved Application
/// State` and restores it on the next launch — and the restored scene never attaches winit's
/// new window: every relaunch rendered blank (egui's viewport stuck at its 10000x10000
/// default) while a fresh install worked. PhotoComfy has no scene state worth restoring, so
/// the saved state is dropped before the UI starts, making every launch the fresh-install
/// path.
fn discard_saved_scene_state() {
    #[cfg(target_os = "ios")]
    if let Some(home) = std::env::var_os("HOME") {
        let _ = std::fs::remove_dir_all(std::path::Path::new(&home).join("Library/Saved Application State"));
    }
}

/// A panic on a device is invisible (stderr goes nowhere), so each one lands in
/// `Documents/crash.log` — readable over Finder file sharing — with a backtrace, before the
/// default hook reports it.
fn install_panic_log() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("PhotoComfy panicked\n{info}\n\n{:?}\n", std::backtrace::Backtrace::force_capture());
        log::error!("{text}");
        if let Some(dir) = documents_dir() {
            use std::io::Write as _;
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("crash.log")) {
                let _ = writeln!(file, "{text}");
            }
        }
        default(info);
    }));
}

/// The app container's Documents folder (Finder file sharing exposes it, and Files › On My
/// iPad › PhotoComfy, which `UIFileSharingEnabled` adds).
fn documents_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("HOME")?).join("Documents");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// The file a Save, Save As or export writes on iPadOS: a real path in the app's `documents`
/// folder, the only one it can write without a security-scoped grant and the one Files shows
/// the user. The save dialog's suggested name (or, when a caller suggests a path elsewhere, its
/// file name) names it. Pure, so a host test can use a temporary directory. A suggestion
/// already directly in that folder keeps its path — a document saved there must save over
/// itself, not a copy beside it. A suggestion with no file name at all falls back to
/// `Untitled.psd`, whose extension the engine's save dialog then keeps.
fn save_path_in(documents: &Path, suggested: &str) -> String {
    let path = Path::new(suggested);
    if path.is_absolute() && path.parent() == Some(documents) {
        return suggested.to_string();
    }
    let name = path.file_name().map_or_else(|| "Untitled.psd".into(), |n| n.to_os_string());
    documents.join(name).to_string_lossy().into_owned()
}

/// Write beside the file and rename, so a crash mid-write can't corrupt the previous save (the
/// same pattern as the desktop's `photocraft_format::atomic_write` and the Android shell's).
fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), String> {
    let path = Path::new(path);
    let name = path.file_name().ok_or("the path names no file")?;
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut tmp = std::ffi::OsString::from(name);
    tmp.push(".pc-tmp");
    let tmp = dir.join(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e.to_string())
        }
    }
}

/// Whether `a` and `b` name one entry in one folder: `same_file` finds they reach the same file
/// in the same folder, and the folder doesn't list both names. Another spelling of a name on
/// iPadOS's case-insensitive volume is the same entry; a hard link is another entry, which a
/// save replaces while the first name keeps the old file. False when the folder can't be read.
fn same_entry(a: &Path, b: &Path) -> bool {
    let folder = |p: &Path| p.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
    let same_file = |x: &Path, y: &Path| {
        use std::os::unix::fs::MetadataExt as _;
        match (std::fs::metadata(x), std::fs::metadata(y)) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    };
    if !same_file(a, b) || !same_file(&folder(a), &folder(b)) {
        return false;
    }
    let (Some(x), Some(y)) = (a.file_name(), b.file_name()) else { return false };
    if x == y {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(folder(a)) else { return false };
    let listed: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.file_name())).collect();
    !(listed.iter().any(|n| n == x) && listed.iter().any(|n| n == y))
}

/// Old saves from the export-sheet design were recorded as a bare file name — the staged tmp
/// file's name, which resolves against the working directory (`/` on a device), so Open Recent
/// never reopened one. Those files now live in Documents; rewrite the bare entries there once
/// at preferences load. An entry with a folder stays as it is, and duplicates (a bare name and
/// its Documents path, say) are dropped keeping the newest.
fn migrate_recent(paths: &mut Vec<String>, documents: &Path) {
    let mut migrated: Vec<String> = Vec::with_capacity(paths.len());
    for path in paths.drain(..) {
        let bare = !path.is_empty() && Path::new(&path).parent().is_none_or(|p| p.as_os_str().is_empty());
        let fixed = if bare { documents.join(&path).to_string_lossy().into_owned() } else { path };
        if !migrated.contains(&fixed) {
            migrated.push(fixed);
        }
    }
    *paths = migrated;
}

/// [`migrate_recent`] on the preferences JSON: the rest of the document is left as it was, and
/// unparseable text (or a list with nothing to rewrite) is returned unchanged.
fn migrate_recent_in_prefs(text: &str, documents: &Path) -> String {
    let Ok(mut prefs) = serde_json::from_str::<serde_json::Value>(text) else { return text.to_string() };
    let Some(list) = prefs.get_mut("fileHandling").and_then(|f| f.get_mut("recentFiles")).and_then(serde_json::Value::as_array_mut) else {
        return text.to_string();
    };
    let mut paths: Vec<String> = list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
    let before = paths.clone();
    migrate_recent(&mut paths, documents);
    if paths == before {
        return text.to_string();
    }
    *list = paths.into_iter().map(serde_json::Value::String).collect();
    serde_json::to_string(&prefs).unwrap_or_else(|_| text.to_string())
}

/// The interface language and CJK script default come from the system locale: the text engine
/// normally reads `PHOTOCRAFT_LOCALE` (`crates/text/src/cjk.rs`), and no launcher sets
/// environment variables for an iPad app. Safe Foundation calls only (objc2's methods are safe
/// wrappers).
#[cfg(target_os = "ios")]
fn install_locale() {
    let identifier = objc2_foundation::NSLocale::currentLocale().languageIdentifier().to_string();
    photocraft_text::cjk::set_shell_locale(identifier);
}

#[cfg(not(target_os = "ios"))]
fn install_locale() {}

/// The smallest logger that gets a startup error onto stderr.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        eprintln!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

/// iPad: the window is the screen, so eframe's window geometry options don't apply. PhotoComfy
/// owns its fallback fonts (`cjk_fonts`, craft-fonts), so eframe's system-font provider stays
/// off, as on the desktop; the saved layout (panel positions) lives beside the preferences.
fn native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("PhotoComfy"),
        system_font_fallback: false,
        persistence_path: app_support_dir().map(|dir| dir.join("ui.ron")),
        ..Default::default()
    };
    // egui's default limits exceed what the iOS simulator GPU allows (max_inter_stage_shader_
    // variables 16 > 15) and its 8192 px texture cap would push big documents to the CPU: request
    // the adapter's own limits, clamped, as the browser build does.
    photocraft_ui_egui::gpu_canvas::use_adapter_limits(&mut options.wgpu_options.wgpu_setup);
    options
}

/// Wraps [`PhotocraftApp`] to hand it each frame, as the web runner does
/// (`apps/photocraft-web/src/web.rs`), with iPad input quirks handled first.
struct IosShell {
    app: PhotocraftApp,
    frames: u32,
    /// Long-press → the synthesized right-click that opens the context menus (long_press.rs).
    long_press: long_press::LongPress,
    /// The pencil's pointer where only the pencil draws (drawing.rs).
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    pencil: drawing::PencilPointer,
    /// `PHOTOCRAFT_DRAW_PROBE`'s replayed touches (draw_probe.rs).
    #[cfg(all(target_os = "ios", feature = "aigc"))]
    probe: Option<draw_probe::Probe>,
    /// Whether that aid is on: it needs a frame every step of the way, and an idle home screen
    /// draws none.
    #[cfg(all(target_os = "ios", feature = "aigc"))]
    probe_on: bool,
}

impl IosShell {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // A painting app keeps the display on while it is in front: the OS auto-locks an idle
        // iPad after two minutes, which reads on a review device as the screen going black with
        // the app still running. Safe UIKit call, main thread by construction here.
        #[cfg(target_os = "ios")]
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            objc2_ui_kit::UIApplication::sharedApplication(mtm).setIdleTimerDisabled(true);
        }
        PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Pro);
        // The copy side of the OS clipboard bridge (src/clipboard.rs): the app's text copies
        // reach the system pasteboard, which egui-winit cannot do on iOS.
        cc.egui_ctx.add_plugin(photocraft_ui_egui::system_clipboard::CopySync::new(clipboard::bridge()));
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services());
        // Who may draw, fill and select here is iOS's one decision (drawing.rs): the Apple Pencil
        // alone on the iPad, nobody on the iPhone. Android makes no such decision.
        app.set_draw_policy(drawing::device_policy());
        // Projects the user keeps in Files › On My iPad › PhotoComfy list as recent files at
        // launch: a save an older build wrote there reopens from Recent without picking it again.
        // Newest first through `push_recent`, which also caps the list per the preferences.
        if let Some(dir) = documents_dir() {
            let documents = ["pcraft", "psd", "psb", "psb", "tif", "tiff", "png", "jpg", "jpeg", "webp", "ora"];
            let mut found: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter_map(|e| {
                            let modified = e.metadata().ok().and_then(|m| m.modified().ok())?;
                            let path = e.path();
                            let ext = path.extension().and_then(|x| x.to_str())?.to_ascii_lowercase();
                            documents.contains(&ext.as_str()).then(|| (modified, path.to_string_lossy().into_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            found.sort_by_key(|(modified, _)| *modified);
            for (_, path) in found {
                if !app.ui.recent_files.contains(&path) {
                    app.push_recent(&path);
                }
            }
        }
        // Testing aid (a launcher sets no CLI args for an iPad app): open a document at start,
        // to reproduce the canvas states a reviewer sees.
        if std::env::var_os("PHOTOCRAFT_OPEN_NEW").is_some()
            && let Err(e) = app.run("file.new", serde_json::json!({"width": 1024, "height": 768, "background": "white"}))
        {
            log::warn!("file.new: {e}");
        }
        // Testing aid: new document then File ▸ Save As at start. A save's whole path — the
        // dialog answer, the write and the document's saved state — is otherwise unreachable
        // without a finger, so this is how a device or simulator run proves a save.
        if std::env::var_os("PHOTOCRAFT_SAVE_NEW").is_some() {
            // `file.new` is an engine command; `file.saveAs` is a shell command (it opens the
            // save dialog), so it goes through the menus dispatcher like the menu item.
            let created = app.run("file.new", serde_json::json!({"width": 512, "height": 512, "name": "SaveProbe", "background": "white"}));
            let saved = created.and_then(|_| photocraft_ui_egui::menus::invoke(&mut app, &cc.egui_ctx, "file.saveAs", serde_json::json!({})));
            if let Err(e) = saved {
                log::warn!("PHOTOCRAFT_SAVE_NEW: {e}");
            }
        }
        // Testing aid: the portrait panel drawer starts open on Layers, for visual checks.
        if std::env::var_os("PHOTOCRAFT_OPEN_DRAWER").is_some() {
            app.touch_drawer = Some(photocraft_ui_egui::dock::Group::Layers);
        }
        // Testing aid: present the system document picker at start (the native sheet is the
        // one thing a screenshot can prove without a finger).
        if std::env::var_os("PHOTOCRAFT_OPEN_PICKER").is_some()
            && let Err(e) = app.run("file.open", serde_json::json!({}))
        {
            log::warn!("file.open: {e}");
        }
        // Testing aid: the embedded CJK fonts, one line each (type_probe.rs).
        #[cfg(all(target_os = "ios", feature = "aigc"))]
        if std::env::var_os("PHOTOCRAFT_FONT_SPECIMEN").is_some() {
            type_probe::font_specimen(&mut app);
        }
        // Long commands and file opens run as background jobs with progress, as on the desktop.
        app.background_jobs = true;
        if let Some(rs) = cc.wgpu_render_state.clone() {
            // The GPU canvas: Metal always, and a device loss is watched, not fatal (see gpu.rs).
            app.set_wgpu(rs);
            app.start_compositor(&cc.egui_ctx);
        }
        Self {
            app,
            frames: 0,
            long_press: long_press::LongPress::default(),
            pencil: drawing::PencilPointer::default(),
            #[cfg(all(target_os = "ios", feature = "aigc"))]
            probe: None,
            #[cfg(all(target_os = "ios", feature = "aigc"))]
            probe_on: std::env::var_os("PHOTOCRAFT_DRAW_PROBE").is_some(),
        }
    }
}

impl eframe::App for IosShell {
    fn on_exit(&mut self) {
        eframe::App::on_exit(&mut self.app);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(target_os = "ios")]
        repair_missing_screen_rect(raw_input);
        #[cfg(target_os = "ios")]
        ime::drain(raw_input);
        // Development builds: `PHOTOCRAFT_DRAW_PROBE` replays finger and pencil touches here, as
        // egui-winit delivers them (draw_probe.rs).
        #[cfg(all(target_os = "ios", feature = "aigc"))]
        if self.probe_on {
            draw_probe::inject(&mut self.app, self.frames, &mut self.probe, raw_input);
        }
        // The iPad's drawing policy (drawing.rs): where only the pencil draws it owns the pointer,
        // so a finger holding it first — the resting hand — neither keeps the pencil from a stroke
        // nor moves it.
        #[cfg(target_os = "ios")]
        {
            use photocraft_ui_egui::draw_policy::DrawPolicy;
            let pencil_only = self.app.draw_policy == DrawPolicy::PencilOnly && self.app.ui.tool.is_drawing();
            let canvas = self.app.last_canvas_rect;
            let on_canvas = |p: egui::Pos2| canvas.contains(p) && ctx.layer_id_at(p).is_none_or(|l| l.order == egui::Order::Background);
            self.pencil.frame(&mut raw_input.events, pencil_only, &on_canvas);
        }
        // A finger held still for half a second is the touch right-click: synthesize it, and
        // swallow the finger's own events until it lifts so the release cannot click through
        // the menu that opened (long_press.rs).
        self.long_press.frame(&mut raw_input.events, raw_input.time);
        if let Some(wait) = self.long_press.fires_within(raw_input.time) {
            ctx.request_repaint_after(wait);
        }
        // A tap can reach the app as both a Touch and an emulated pointer press; keep one, or
        // the twin closes the menu the first one opened (#2095).
        photocraft_ui_egui::touch_twins::drop_twins(ctx, raw_input);
        // One pinch, two channels: winit's iOS system recognizer delivers it as `Event::Zoom`,
        // and the same fingers arrive as Touch events that egui's `zoom_delta` already reports.
        // Keeping both double-applies the pinch, and the recognizer's trailing events keep
        // zooming after the fingers lift — the endless zoom wobble. The touch channel is the
        // accurate one, so the recognizer's zooms are dropped (a trackpad pinch on an iPad is
        // the one input that loses its zoom; its scroll still pans).
        if self.app.session.prefs().interface.touch_ui {
            raw_input.events.retain(|event| !matches!(event, egui::Event::Zoom(_)));
        }
        // `PHOTOCRAFT_DEBUG_IOS`: every touch press lands in the log with its egui position and
        // the pointer position the same frame synthesized — comparing the two with where the
        // finger actually was names any input offset (the "tap lands a row off" diagnosis).
        if std::env::var_os("PHOTOCRAFT_DEBUG_IOS").is_some() {
            for event in &raw_input.events {
                match event {
                    egui::Event::Touch { phase: egui::TouchPhase::Start, pos, .. } => {
                        breadcrumb(&format!(
                            "touch press at {pos:?} (zoom {:.3} ppp {:.3}, screen {:?})",
                            ctx.zoom_factor(),
                            ctx.pixels_per_point(),
                            ctx.input(|i| i.content_rect())
                        ));
                    }
                    egui::Event::PointerButton { pressed: true, pos, .. } => {
                        breadcrumb(&format!(
                            "pointer press at {pos:?} (zoom {:.3} ppp {:.3}, screen {:?})",
                            ctx.zoom_factor(),
                            ctx.pixels_per_point(),
                            ctx.input(|i| i.content_rect())
                        ));
                        LAST_TAP.with(|t| *t.borrow_mut() = Some(*pos));
                    }
                    _ => {}
                }
            }
        }
        eframe::App::raw_input_hook(&mut self.app, ctx, raw_input);
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.frames == 0 {
            breadcrumb("first frame");
            #[cfg(target_os = "ios")]
            unsafe extern "C" {
                safe fn pc_install_text_input();
            }
            #[cfg(target_os = "ios")]
            pc_install_text_input();
        }
        // `PHOTOCRAFT_DEBUG_IOS`: the view/layer/drawable geometry once, to name any present-path
        // mismatch (the anisotropic-stretch diagnosis).
        if std::env::var_os("PHOTOCRAFT_DEBUG_IOS").is_some() && (self.frames == 2 || self.frames.is_multiple_of(30)) {
            #[cfg(target_os = "ios")]
            unsafe extern "C" {
                safe fn pc_log_geometry();
            }
            #[cfg(target_os = "ios")]
            unsafe {
                pc_log_geometry();
            }
            let vp = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect, i.viewport().monitor_size));
            breadcrumb(&format!("viewport inner={vp:?}"));
        }
        self.frames += 1;
        // `PHOTOCRAFT_DEBUG_IOS=1`: a throttled sample of everything the touch layout and the
        // canvas zoom read, for the wobble/crash diagnoses (a device has no visible stderr; use
        // the log next to the breadcrumbs).
        if std::env::var_os("PHOTOCRAFT_DEBUG_IOS").is_some() && self.frames.is_multiple_of(30) {
            let screen = ctx.input(|i| i.content_rect());
            let insets = ctx.input(|i| i.safe_area_insets().0);
            let view = self.app.ui.views.first().map(|v| (v.zoom, v.center));
            let forced = std::env::var_os("PHOTOCRAFT_FORCE_PORTRAIT").is_some();
            let touch = self.app.session.prefs().interface.touch_ui;
            breadcrumb(&format!(
                "frame {} ppp {:.3} zoom {:.3} screen {}x{} insets t{} b{} view {view:?} forced={forced} touch_ui={touch}",
                self.frames,
                ctx.pixels_per_point(),
                ctx.zoom_factor(),
                screen.width(),
                screen.height(),
                insets.top,
                insets.bottom,
            ));
        }
        self.app.logic_frame(ctx, frame);
        self.app.finish_type_reset_logic(ctx);
        // A replayed touch generates no event of its own: keep the frames coming (draw_probe.rs).
        #[cfg(all(target_os = "ios", feature = "aigc"))]
        if self.probe_on {
            ctx.request_repaint();
        }
        // Testing aid: replay a pinyin keyboard session through the text input view (type_probe.rs).
        #[cfg(all(target_os = "ios", feature = "aigc"))]
        if std::env::var_os("PHOTOCRAFT_TYPE_PROBE").is_some() {
            type_probe::frame(self, ctx);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui_frame(ui, frame);
        self.app.finish_type_reset_pass(ui.ctx());
        // `PHOTOCRAFT_DEBUG_IOS`: a crosshair at the position the last tap was RECEIVED at,
        // for ~3 s — one screenshot then shows the finger's intent and the app's belief.
        if std::env::var_os("PHOTOCRAFT_DEBUG_IOS").is_some() {
            // A 100 pt grid of labelled dots: tap one, and the crosshair's distance to the dot
            // you aimed at is the input offset, read straight off the screenshot.
            let screen = ui.input(|i| i.content_rect());
            let p = ui.ctx().debug_painter();
            let mut x = 100.0;
            while x < screen.right() {
                let mut y = 100.0;
                while y < screen.bottom() {
                    let at = egui::pos2(x, y);
                    p.circle_filled(at, 3.0, egui::Color32::from_rgb(0, 200, 255));
                    p.text(
                        at + egui::vec2(4.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{x:.0},{y:.0}"),
                        egui::FontId::monospace(11.0),
                        egui::Color32::from_rgb(0, 170, 255),
                    );
                    y += 100.0;
                }
                x += 100.0;
            }
        }
        if std::env::var_os("PHOTOCRAFT_DEBUG_IOS").is_some()
            && let Some(pos) = LAST_TAP.with(|t| *t.borrow())
        {
            let p = ui.ctx().debug_painter();
            p.line_segment([pos - egui::vec2(40.0, 0.0), pos + egui::vec2(40.0, 0.0)], egui::Stroke::new(3.0, egui::Color32::RED));
            p.line_segment([pos - egui::vec2(0.0, 40.0), pos + egui::vec2(0.0, 40.0)], egui::Stroke::new(3.0, egui::Color32::RED));
            p.circle_filled(pos, 6.0, egui::Color32::YELLOW);
            p.text(pos + egui::vec2(8.0, 8.0), egui::Align2::LEFT_TOP, format!("{pos:?}"), egui::FontId::monospace(14.0), egui::Color32::RED);
        }
    }
}

#[cfg(target_os = "ios")]
fn repair_missing_screen_rect(raw_input: &mut egui::RawInput) {
    let invalid = raw_input.screen_rect.is_none_or(|rect| rect.width() >= 9999.0 || rect.height() >= 9999.0);
    if !invalid {
        return;
    }

    let Some(mtm) = objc2::MainThreadMarker::new() else {
        return;
    };
    let scenes = objc2_ui_kit::UIApplication::sharedApplication(mtm).connectedScenes().allObjects();
    let scene_size = (0..scenes.count()).find_map(|index| {
        scenes.objectAtIndex(index).downcast::<objc2_ui_kit::UIWindowScene>().ok().map(|scene| {
            #[allow(deprecated)]
            let bounds = scene.coordinateSpace().bounds();
            egui::vec2(bounds.size.width as f32, bounds.size.height as f32)
        })
    });
    #[allow(deprecated)]
    let fallback_bounds = objc2_ui_kit::UIScreen::mainScreen(mtm).bounds();
    let size = scene_size.unwrap_or_else(|| egui::vec2(fallback_bounds.size.width as f32, fallback_bounds.size.height as f32));
    if size.is_finite() && size.x > 0.0 && size.y > 0.0 {
        raw_input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size));
    }
}

fn services() -> Services {
    let mut services = services_in(documents_dir());
    if let Some(dir) = app_support_dir() {
        attach_app_storage(&mut services, &dir);
    }
    services
}

/// Global imports live in the app container, not beside the security-scoped source file.
/// Kept separate from `services_in`: host tests opt into a temporary root explicitly instead of
/// opening the developer's real preset store when they only need the document services.
fn attach_app_storage(services: &mut Services, app_support: &Path) {
    services.preset_store = Some(photocraft_engine::preset_store::open_dir_async(app_support.join("Presets")));
    services.comfy_workflows_dir = Some(app_support.join("comfy_workflows"));
    services.plugin_import_dir = Some(app_support.join("Plugins"));
}

/// [`services`] with the folder a save writes into, so a host test can point it at a temporary
/// directory (`services()` uses the container's Documents). `None` keeps the suggested path:
/// there is nowhere in the container to put a real file.
fn services_in(documents: Option<PathBuf>) -> Services {
    #[cfg(feature = "aigc")]
    let (comfy_transport, comfy_image_api) = (Some(api_gen::transport()), true);
    #[cfg(not(feature = "aigc"))]
    let (comfy_transport, comfy_image_api) = (None, false);
    #[cfg(feature = "aigc")]
    let comfy_generations_dir = app_support_dir().map(|dir| dir.join("Generations"));
    #[cfg(not(feature = "aigc"))]
    let comfy_generations_dir = None;

    Services {
        import: Some(Box::new(|name: &str, bytes: &[u8], opts: &photocraft_io::ImportOptions| {
            photocraft_io::import_with_options(name, bytes, opts).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        export: Some(Box::new(|doc: &Document, path: &str, settings: &photocraft_ui_egui::ExportSettings| {
            photocraft_io::export(doc, path, &settings.io_options()).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())
        })),
        // The system document picker (ios/PCDocumentPicker.m + src/picker.rs). Open reads the
        // picked files in the delegate (security scope) and answers with contents. Save answers
        // a REAL path in the app's Documents at once: that folder is Files › On My iPad ›
        // PhotoComfy (UIFileSharingEnabled, PhotoComfy/Info.plist), so Save writes back in
        // place, Open Recent can reopen the file and the overwrite prompt has a stamp. The old
        // export-sheet answer was a bare staged name, which pointed at nothing reachable.
        file_dialog: Some(Box::new(move |request, _parent, reply| match request {
            FileDialogRequest::Open { multiple, .. } => picker::open(multiple, reply),
            // Folder pickers have no counterpart (as on the web): back out.
            FileDialogRequest::Folder { .. } => reply.send(None),
            FileDialogRequest::Save { suggested } => {
                let path = match &documents {
                    Some(dir) => save_path_in(dir, &suggested),
                    None => suggested,
                };
                reply.send(Some(FileDialogAnswer::SaveTo(path)));
            }
            // Save As asks the system for the destination; ordinary Save keeps the app's
            // Documents path above, so it still writes back in place.
            FileDialogRequest::SaveLocation { suggested } => picker::save_location(&suggested, reply),
        })),
        write: Some(Box::new(|path: &str, bytes: &[u8]| write_atomic(path, bytes))),
        file_stamp: Some(Box::new(|path: &str| {
            let meta = std::fs::metadata(path).ok()?;
            Some(FileStamp { len: meta.len(), modified: meta.modified().ok() })
        })),
        same_file: Some(Box::new(|a: &str, b: &str| same_entry(Path::new(a), Path::new(b)))),
        // The answer above is immediate: no native panel asks whether to replace the file.
        save_dialog_unconfirmed: true,
        encode_png: Some(Box::new(|w, h, rgba| {
            let img = Image::from_u8(w, h, ChannelLayout::Rgba, rgba.to_vec()).map_err(|e| e.to_string())?;
            photocraft_codecs::encode(&img, photocraft_codecs::Format::Png, &EncodeOptions::default()).map_err(|e| e.to_string())
        })),
        load_prefs: Some(Box::new(read_prefs)),
        save_prefs: Some(Box::new(write_prefs)),
        // Generation goes straight to the user's OpenAI-compatible Image API (src/api_gen.rs),
        // not through a ComfyUI server: the endpoint, model and key are Preferences ▸ Integrations
        // (editable in the generation panel), and results persist in the container like the
        // desktop's Generations folder, so the shelf survives a relaunch.
        // The OS text clipboard bridge (src/clipboard.rs): paste reads UIPasteboard, so text
        // copied in other apps pastes; copies mirror out through the CopySync plugin.
        system_clipboard: Some(clipboard::bridge()),
        comfy_transport,
        comfy_image_api,
        comfy_generations_dir,
        ..Default::default()
    }
}

const PREFS_FILE: &str = "preferences.json";

/// The app container's Application Support folder. The iOS sandbox's `HOME` is the container
/// (the simulator's likewise); created on first use, `None` where storage is unavailable.
/// The app was born PhotoCraft: its first home (preferences, the Generations folder) moves
/// here once, so the rename keeps what the user configured.
fn app_support_dir() -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    let dir = base.join("PhotoComfy");
    let old = base.join("PhotoCraft");
    if !dir.exists() && old.is_dir() && std::fs::rename(&old, &dir).is_err() {
        std::fs::create_dir_all(&dir).ok()?;
    }
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn read_prefs() -> Option<String> {
    let text = std::fs::read_to_string(app_support_dir()?.join(PREFS_FILE)).ok()?;
    #[cfg(not(feature = "aigc"))]
    {
        // A build without `aigc` can share a container with a build that saved Image API
        // settings. It must not consume or rewrite those settings.
        let mut value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
        if let Some(integrations) = value.get_mut("integrations").and_then(serde_json::Value::as_object_mut) {
            integrations.remove("imageApiBase");
            integrations.remove("imageApiKey");
            integrations.remove("imageApiModel");
        }
        serde_json::to_string(&value).ok()
    }
    #[cfg(feature = "aigc")]
    // The rows the export-sheet design left pointing at a bare name get their real Documents
    // path now (see `migrate_recent`). No Documents folder: the text stays as it was.
    {
        Some(match documents_dir() {
            Some(dir) => migrate_recent_in_prefs(&text, &dir),
            None => text,
        })
    }
}

fn write_prefs(text: &str) -> Result<(), String> {
    let dir = app_support_dir().ok_or("no Application Support directory")?;
    // Write beside the file and rename, so a crash mid-write can't corrupt the preferences.
    let tmp = dir.join(format!("{PREFS_FILE}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join(PREFS_FILE)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the container's Documents (the tests never touch the real one).
    fn documents() -> PathBuf {
        PathBuf::from("/var/mobile/Containers/Data/Application/PC/Documents")
    }

    #[test]
    fn a_save_name_lands_in_documents_and_a_documents_path_is_kept() {
        let dir = documents();
        let in_documents = |name: &str| dir.join(name).to_string_lossy().into_owned();
        assert_eq!(save_path_in(&dir, "Untitled-1.psd"), in_documents("Untitled-1.psd"));
        // The document's own path, which Save As suggests for an already-saved file, must save
        // over that file rather than beside it.
        let own = dir.join("Painting.psd");
        assert_eq!(save_path_in(&dir, own.to_str().unwrap()), own.to_string_lossy());
        // A suggestion from anywhere else keeps only its name: the app may write nowhere else.
        assert_eq!(save_path_in(&dir, "/tmp/作品/Other.PSD"), in_documents("Other.PSD"));
        assert_eq!(save_path_in(&dir, "作品/My image.psd"), in_documents("My image.psd"));
        // No file name at all still produces a writable path (the engine adds the extension).
        assert_eq!(save_path_in(&dir, "/"), in_documents("Untitled.psd"));
    }

    #[test]
    fn old_bare_recent_names_move_to_documents_once() {
        let dir = documents();
        let in_documents = dir.join("Untitled.psd").to_string_lossy().into_owned();
        let mut paths = vec![
            "Untitled.psd".to_string(),
            "/tmp/elsewhere.psd".to_string(),
            in_documents.clone(), // the same row after migration: the duplicate is dropped
            "sub/name.psd".to_string(),
        ];
        migrate_recent(&mut paths, &dir);
        assert_eq!(paths, vec![in_documents, "/tmp/elsewhere.psd".to_string(), "sub/name.psd".to_string()]);
    }

    #[test]
    fn preferences_migration_rewrites_only_the_recent_list() {
        let dir = documents();
        let text = r#"{"version":1,"fileHandling":{"recentFileCount":20,"recentFiles":["旧.psd","/tmp/abs.psd"]},"interface":{"language":"zh"}}"#;
        let value: serde_json::Value = serde_json::from_str(&migrate_recent_in_prefs(text, &dir)).unwrap();
        assert_eq!(value["fileHandling"]["recentFileCount"], 20);
        assert_eq!(value["interface"]["language"], "zh");
        assert_eq!(value["fileHandling"]["recentFiles"][0], dir.join("旧.psd").to_string_lossy().as_ref());
        assert_eq!(value["fileHandling"]["recentFiles"][1], "/tmp/abs.psd");
        // Nothing to rewrite, or text that isn't preferences: exactly as it came in.
        let clean = r#"{"fileHandling":{"recentFiles":["/tmp/abs.psd"]}}"#;
        assert_eq!(migrate_recent_in_prefs(clean, &dir), clean);
        assert_eq!(migrate_recent_in_prefs("not json", &dir), "not json");
    }

    /// The shell's own services, driven end to end on the host: Save As of a new document writes
    /// a real PSD into the save folder and records that path, so Save writes back to it and the
    /// recent list names a file that can be reopened. With the old bare-name answer the bytes
    /// went to a tmp file, the document's path was a name and every assertion here failed.
    #[test]
    fn a_save_as_writes_a_real_file_the_document_and_recent_list_name() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("photocraft-ios-save-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut services = services_in(Some(dir.clone()));
        // Never read or write the developer's real preferences from a test.
        services.load_prefs = None;
        services.save_prefs = None;
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        app.run("file.new", serde_json::json!({"width": 16, "height": 16, "name": "Untitled-1", "background": "white"})).unwrap();
        app.save_as(None).unwrap();
        app.poll_file_dialog(&egui::Context::default(), None);

        let path = dir.join("Untitled-1.psd").to_string_lossy().into_owned();
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("no save at {path}: {e} (status: {})", app.ui.status));
        assert_eq!(&bytes[..4], b"8BPS", "the file is a PSD");
        let st = app.session.active().unwrap();
        assert_eq!(st.path.as_deref(), Some(path.as_str()), "the document owns the written file");
        assert_eq!(st.doc.name, "Untitled-1.psd");
        assert_eq!(st.saved_revision, st.revision, "a confirmed write leaves nothing unsaved");
        assert_eq!(app.ui.recent_files.first().map(String::as_str), Some(path.as_str()));

        // File ▸ Save writes the same file back in place (no dialog).
        photocraft_ui_egui::menus::invoke(&mut app, &egui::Context::default(), "file.save", serde_json::json!({})).unwrap();
        assert!(!app.file_dialog_open());
        // The recent entry names a file that reopens: close the document, open from its path.
        app.run("file.close", serde_json::json!({})).unwrap();
        assert!(app.session.active().is_none());
        app.open_paths(std::slice::from_ref(&path));
        assert_eq!(app.session.active().and_then(|st| st.path.as_deref()), Some(path.as_str()), "the saved file opens again");

        // iPadOS has no save panel to ask in: a second Save As onto the existing name asks
        // through the app's replace prompt instead of overwriting silently.
        let first = std::fs::read(&path).unwrap();
        app.run("file.new", serde_json::json!({"width": 8, "height": 8, "name": "Untitled-1", "background": "white"})).unwrap();
        app.save_as(None).unwrap();
        app.poll_file_dialog(&egui::Context::default(), None);
        assert!(app.file_dialog_open(), "the save waits for the replace answer");
        assert_eq!(std::fs::read(&path).unwrap(), first, "nothing is written before the answer");
        let id = app.ui.dialogs.last().map(|d| d.id).expect("the replace prompt is open");
        photocraft_ui_egui::dialogs::confirm(&mut app, id).unwrap();
        assert_ne!(std::fs::read(&path).unwrap(), first, "Replace writes the new document");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The production storage wiring, driven with a temporary Application Support root: an
    /// image-imported brush (including its sampled tip), a pattern and a recorded action all
    /// survive dropping the Session and reopening the on-disk store.
    #[test]
    fn global_brush_pattern_and_action_imports_reopen_from_app_storage() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let root = std::env::temp_dir().join(format!("photocraft-ios-presets-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut services = services_in(None);
        services.load_prefs = None;
        services.save_prefs = None;
        attach_app_storage(&mut services, &root);
        assert_eq!(services.comfy_workflows_dir.as_deref(), Some(root.join("comfy_workflows").as_path()));
        assert_eq!(services.plugin_import_dir.as_deref(), Some(root.join("Plugins").as_path()));

        let opened = services.preset_store.take().unwrap().recv_timeout(std::time::Duration::from_secs(30)).unwrap();
        let mut session = photocraft_engine::Session::new();
        assert!(session.attach_preset_store(opened).is_empty());
        session.execute("file.new", serde_json::json!({"width": 8, "height": 8, "background": "white"})).unwrap();
        session.execute("edit.fill", serde_json::json!({"color": "#000000"})).unwrap();
        let png = photocraft_io::export(&session.active().unwrap().doc, "Stamp.png", &photocraft_io::ExportOptions::default()).unwrap().bytes;
        session
            .execute("brush.presets.importImage", serde_json::json!({"data": photocraft_engine::paint::tile::b64_encode(&png), "fileName": "Stamp.png"}))
            .unwrap();
        session.execute("edit.definePattern", serde_json::json!({"name": "Kept Pattern"})).unwrap();
        session.execute("actions.record", serde_json::json!({"name": "Kept Action"})).unwrap();
        session.execute("layer.new.layer", serde_json::json!({})).unwrap();
        session.execute("actions.stop", serde_json::json!({})).unwrap();
        drop(session);

        let mut restored = photocraft_engine::Session::new();
        assert!(restored.attach_preset_store(photocraft_engine::preset_store::open_dir(root.join("Presets"))).is_empty());
        assert!(restored.tools.presets.iter().any(|preset| preset.name == "Stamp"));
        assert!(restored.patterns.items.iter().any(|pattern| pattern.name == "Kept Pattern"));
        let actions = restored.execute("actions.list", serde_json::json!({})).unwrap();
        assert!(actions["actions"].as_array().unwrap().iter().any(|action| action["name"] == "Kept Action"));
        assert!(root.join("Presets").join("actions.json").is_file());
        assert!(std::fs::read_dir(root.join("Presets").join("tips")).unwrap().count() > 0, "the sampled brush tip is on disk");
        let _ = std::fs::remove_dir_all(&root);
    }
}
