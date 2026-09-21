// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Low-level layout API for advanced integrations. Semver not guaranteed.

pub use crate::dock::builder::compile::{
    active_panel_in_pane, build_tree, first_pane, owning_pane, pane_for_panel, BuiltLayout,
};
pub use crate::dock::builder::DockIndex;
pub use crate::dock::factory::Factory;
pub use crate::dock::manager::{DockManager, DragSession, DropZone, TabBarTarget};
pub use crate::dock::widget::{dispatch_action, finish_drag, DockAction, TabAction};
