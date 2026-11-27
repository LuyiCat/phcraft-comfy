//! Whether this build offers image generation: the ComfyUI / Image API panel, its menu items and
//! its commands. The shell alone decides, as it does the drawing policy (`input/draw_policy.rs`).
//! A shell whose build leaves the generation transport out of the binary switches it off once at
//! start (`PhotocraftApp::set_generation_removed`); the interface then names no generation
//! command — no Generate button in the portrait drawer, no File or Window menu item, nothing in
//! the lists that feed the palette and the shortcut editor — and `menus::invoke` refuses the ids.
//! Every other shell keeps the default and its behaviour.

/// Image generation in this build.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Generation {
    /// Offered as it always was: the shell wires a transport, or leaves the entries in place
    /// where the engine refuses them (the web, the host test harnesses).
    #[default]
    Offered,
    /// Left out of this build.
    Removed,
}

impl Generation {
    pub fn offered(self) -> bool {
        self == Self::Offered
    }

    /// Whether `id` is a generation command: the engine's `ai.comfyui.*` ids and Window ›
    /// ComfyUI Generation.
    pub fn owns(id: &str) -> bool {
        id.starts_with("ai.comfyui.") || id == "window.panel.comfy"
    }
}
