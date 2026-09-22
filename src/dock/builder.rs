// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Declarative layout trees and [`DockSession`].

pub mod compile;
mod index;
mod session;
mod spec;

pub use index::DockIndex;
pub use session::{DockSession, InitialFocus, PaneTarget, PanelCycle};
pub use spec::{
    horizontal, panel, panel_def, single, tabs, vertical, DockSpec, LayoutArea, LayoutTree,
    PanelDef, SplitNode, TabsNode,
};
pub use compile::{build_area, build_tree, BuiltLayout};
