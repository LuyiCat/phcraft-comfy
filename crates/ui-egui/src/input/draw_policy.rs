//! Who may draw, fill and select on the canvas ([`PhotocraftApp::draw_policy`]).
//!
//! The shell decides, and nothing else does: the iPad's policy is the Apple Pencil alone and the
//! iPhone's is nobody (`apps/photocraft-ios/src/draw_policy.rs`), while Android, the desktop and
//! the web keep the default, any input. There is no platform test in the drawing logic itself, so
//! a change for one shell cannot reach another. The canvas asks the policy before a drawing
//! gesture starts (`canvas/tool_input.rs`, Liquify, Select and Mask's Refine Edge Brush), the
//! toolbar, the tool commands and the menus ask it for what they offer, and the touch arbitration
//! for whether a finger stands a pencil's gesture down.

use crate::state::Tool;

/// Who may draw, fill and select on the canvas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrawPolicy {
    /// Any pointer source: a mouse, a stylus and a finger (desktop, web, Android).
    #[default]
    AnyInput,
    /// Only the stylus, a touch contact that reports its pressure (the Apple Pencil on iPad): a
    /// finger never draws, sprays, erases, fills or selects.
    PencilOnly,
    /// Nobody: this device has no drawing, fill or selection at all (iPhone).
    None,
}

/// Why a press must not start a drawing gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A finger pressed where only the pencil draws.
    NeedsPencil,
    /// Nothing draws on this device.
    NoDrawing,
}

impl Refusal {
    /// The status-bar hint for the refused press.
    pub fn hint(self) -> &'static str {
        match self {
            Self::NeedsPencil => "Painting needs the Apple Pencil",
            Self::NoDrawing => "Drawing, filling and selecting are not available on this device",
        }
    }
}

impl DrawPolicy {
    /// Why a press with `tool` must not start. `finger`: it comes from a touch contact that began
    /// without a force (`FingerPresses::pointer_is_finger`). `None` lets it start.
    pub fn refusal(self, tool: Tool, finger: bool) -> Option<Refusal> {
        tool.is_drawing().then(|| self.drawing_refusal(finger)).flatten()
    }

    /// [`Self::refusal`] for a gesture that paints without a tool of its own (Liquify, the
    /// Refine Edge Brush).
    pub fn painting_refusal(self, finger: bool) -> Option<Refusal> {
        self.drawing_refusal(finger)
    }

    fn drawing_refusal(self, finger: bool) -> Option<Refusal> {
        match self {
            Self::AnyInput => None,
            Self::PencilOnly => finger.then_some(Refusal::NeedsPencil),
            Self::None => Some(Refusal::NoDrawing),
        }
    }

    /// Whether the toolbar and the tool commands offer `tool`.
    pub fn offers(self, tool: Tool) -> bool {
        self != Self::None || !tool.is_drawing()
    }

    /// Whether the menus offer command `id`: under [`Self::None`] no selection, fill or Liquify
    /// command (the Select menu, Edit › Fill, Stroke and Content-Aware Fill, Layer › New Fill
    /// Layer, Liquify).
    pub fn offers_command(self, id: &str) -> bool {
        self != Self::None
            || !(id.starts_with("select.")
                || id.starts_with("layer.newFillLayer.")
                || matches!(
                    id,
                    "edit.fill" | "edit.stroke" | "edit.contentAwareFill" | "filter.liquify" | "tools.increaseBrushHardness" | "tools.decreaseBrushHardness"
                ))
    }

    /// Whether a finger joining the stylus's gesture stands it down. Only under [`Self::PencilOnly`]
    /// it does not: there a finger is never drawing input, so one that lands mid-stroke (the
    /// resting hand) neither cancels the stroke nor ends it.
    pub fn finger_joins_pencil_stand_down(self) -> bool {
        self != Self::PencilOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_input_never_refuses_and_offers_everything() {
        for tool in Tool::ALL {
            assert_eq!(DrawPolicy::AnyInput.refusal(tool, true), None, "{tool:?}");
            assert!(DrawPolicy::AnyInput.offers(tool), "{tool:?}");
        }
        assert!(DrawPolicy::AnyInput.offers_command("select.all"));
        assert!(DrawPolicy::AnyInput.finger_joins_pencil_stand_down());
    }

    #[test]
    fn pencil_only_refuses_a_finger_for_drawing_tools_only() {
        for tool in Tool::ALL {
            let want = tool.is_drawing().then_some(Refusal::NeedsPencil);
            assert_eq!(DrawPolicy::PencilOnly.refusal(tool, true), want, "finger {tool:?}");
            assert_eq!(DrawPolicy::PencilOnly.refusal(tool, false), None, "pencil {tool:?}");
            assert!(DrawPolicy::PencilOnly.offers(tool), "{tool:?}");
        }
        assert_eq!(DrawPolicy::PencilOnly.painting_refusal(true), Some(Refusal::NeedsPencil));
        assert!(DrawPolicy::PencilOnly.offers_command("select.all"));
        assert!(!DrawPolicy::PencilOnly.finger_joins_pencil_stand_down());
    }

    #[test]
    fn nobody_refuses_every_source_and_offers_no_drawing() {
        for tool in Tool::ALL {
            let want = tool.is_drawing().then_some(Refusal::NoDrawing);
            assert_eq!(DrawPolicy::None.refusal(tool, true), want, "finger {tool:?}");
            assert_eq!(DrawPolicy::None.refusal(tool, false), want, "pencil {tool:?}");
            assert_eq!(DrawPolicy::None.offers(tool), !tool.is_drawing(), "{tool:?}");
        }
        assert_eq!(DrawPolicy::None.painting_refusal(false), Some(Refusal::NoDrawing));
    }

    #[test]
    fn nobody_offers_no_selection_fill_or_liquify_command() {
        for id in [
            "select.all",
            "select.deselect",
            "select.inverse",
            "select.colorRange",
            "select.selectAndMask",
            "select.transformSelection",
            "edit.fill",
            "edit.stroke",
            "edit.contentAwareFill",
            "layer.newFillLayer.solidColor",
            "filter.liquify",
        ] {
            assert!(!DrawPolicy::None.offers_command(id), "{id}");
            assert!(DrawPolicy::PencilOnly.offers_command(id), "{id}");
        }
        for id in [
            "edit.copy",
            "edit.paste",
            "edit.freeTransform",
            "layer.new.layer",
            "view.zoomIn",
            "tools.select.brush",
            "file.save",
            "image.adjustments.levels",
            "filter.gaussianBlur",
        ] {
            assert!(DrawPolicy::None.offers_command(id), "{id}");
        }
    }
}
