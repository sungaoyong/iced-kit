// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

mod layout;
mod pane;
mod proportional;
mod region;

pub use layout::*;
pub use pane::*;
pub use proportional::ProportionalGroup;
pub use region::{
    CLOSED_BOTTOM_STRIP, Dock, DockPlacement, DockRegion, DockRegions, PANEL_MIN_SIZE,
};
