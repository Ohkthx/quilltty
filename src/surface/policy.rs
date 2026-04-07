//! File: src/surface/policy.rs

use crate::{geom::Point, surface::pane::PaneElement};

/// Action resolved from a pane hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneAction {
    None,
    FocusOnly,
    BeginMove { grab_offset: Point },
    BeginResize,
}

/// Interaction policy applied to a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanePolicy {
    pub movable: bool,
    pub resizable: bool,
    pub focus_on_decor_press: bool,
}

impl Default for PanePolicy {
    fn default() -> Self {
        Self {
            movable: true,
            resizable: true,
            focus_on_decor_press: true,
        }
    }
}

impl PanePolicy {
    /// Resolves the action that should occur for the given pane element hit.
    #[inline]
    pub(crate) fn action_for_hit(&self, element: PaneElement, local: Point) -> PaneAction {
        match element {
            PaneElement::Content => PaneAction::None,

            PaneElement::Resize if self.resizable => PaneAction::BeginResize,

            PaneElement::Title | PaneElement::Border if self.movable => {
                PaneAction::BeginMove { grab_offset: local }
            }

            PaneElement::Title | PaneElement::Border | PaneElement::Resize
                if self.focus_on_decor_press =>
            {
                PaneAction::FocusOnly
            }

            _ => PaneAction::None,
        }
    }
}
