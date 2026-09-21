// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Common imports for application code.
//!
//! ```ignore
//! use iced_dock::prelude::*;
//! ```

pub use crate::dock::builder::{
    horizontal, panel, single, tabs, vertical, DockSession, InitialFocus, LayoutTree, PaneTarget,
    PanelCycle, PanelDef,
};
pub use crate::dock::spatial::Direction;
pub use crate::dock::style::{default, preset, Catalog, DockStyle, PaneContent, StyleFn};
pub use crate::dock::widget::{dock, Dock, DockEvent, DockWidgetState, TabBarScrollbarAttachment};
pub use crate::dock::{Error, Result};
