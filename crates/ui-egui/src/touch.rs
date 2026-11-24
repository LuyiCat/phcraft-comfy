//! Touch Interface layout helpers: the points where a touch-first shell differs from the
//! pointer-first desktop one, decided from the preference and the screen at hand.

use crate::PhotocraftApp;

/// The desktop layout's smallest window (`apps/photocraft` native_options). The Touch Interface
/// scale shrinks towards 1 when the magnified UI would leave less than this to lay out in, where
/// dialogs and the dock stop fitting (# an 11-inch iPad's 834 pt portrait at ×1.4 would leave
/// 595).
pub const MIN_DESIGN_WIDTH: f32 = 760.0;
pub const MIN_DESIGN_HEIGHT: f32 = 480.0;

/// Touch Interface is on (`interface.touchUi`).
pub fn on(app: &PhotocraftApp) -> bool {
    app.session.prefs().interface.touch_ui
}

/// The screen is taller than wide (a portrait iPad or Android handset). Decides the mobile
/// chrome: the tools row above the canvas and the panel drawer below it, in place of the
/// desktop's side toolbar and end dock. Portrait chrome ships on the mobile shells only — the
/// desktop app and the web build keep the desktop layout whatever shape the window takes, so a tall desktop window with
/// Touch Interface on still shows the side toolbar and end dock. `PHOTOCRAFT_FORCE_PORTRAIT`
/// exercises the portrait chrome on any host (a testing aid; nothing in the app sets it), and
/// `cfg(test)` lets the host test harnesses drive it without the variable.
pub fn portrait(app: &PhotocraftApp, ctx: &egui::Context) -> bool {
    if std::env::var_os("PHOTOCRAFT_FORCE_PORTRAIT").is_some() {
        return on(app);
    }
    on(app) && portrait_platform() && {
        let screen = ctx.input(|i| i.content_rect()).size();
        screen.y > screen.x
    }
}

/// Where the portrait chrome runs: iOS and Android builds. Desktop and web never (`cfg(test)`
/// stands in for the mobile shells on the host the tests run on).
fn portrait_platform() -> bool {
    cfg!(any(target_os = "ios", target_os = "android", test))
}
