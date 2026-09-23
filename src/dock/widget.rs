// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Custom iced widgets for docking.

mod action;
mod area;
mod controls;
mod compose;
mod dock;
mod event;
mod split;
mod state;
mod tab_dock;
mod tab_strip;
mod title_drag;

pub use crate::dock::style::PaneContent;
pub use action::{DockAction, TabAction};
pub use dock::{dock, Dock, DockBuilder, TabBarScrollbarAttachment};
pub use controls::MenuEntry;
pub use event::DockEvent;
pub use state::{dispatch_action, finish_drag, DockWidgetState};