// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Low-level layout API for advanced integrations. Semver not guaranteed.

pub use crate::dock::builder::compile::{
    active_panel_in_pane, build_area, build_tree, displayed_panel, first_pane, owning_pane,
    pane_for_panel, pane_is_empty, panel_is_visible, panels_in_tree, visible_tabs, BuiltLayout,
};
pub use crate::dock::builder::DockIndex;
pub use crate::dock::factory::Factory;
pub use crate::dock::manager::{DockManager, DragSession, DropZone, TabBarTarget};
pub use crate::dock::model::{
    Dock, DockPlacement, DockRegion, DockRegions, CLOSED_BOTTOM_STRIP, PANEL_MIN_SIZE,
};
pub use crate::dock::widget::{dispatch_action, finish_drag, DockAction, TabAction};
