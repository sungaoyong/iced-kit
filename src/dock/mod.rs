//! Docking layout system for iced.
//!
//! ## Quick start
//!
//! ```ignore
//! use iced_dock::prelude::*;
//!
//! let session = DockSession::from_tree(layout_tree())?;
//! dock::<Message>()
//!     .state(session.state())
//!     .on_event(Message::DockEvent)
//!     .content(|key| view_panel(key))
//!     .build();
//! ```
//!
//! The dock widget applies layout mutations internally. Handle [`DockEvent`] in `update` for
//! side effects only — do not call [`DockSession::dispatch`] for widget-originated input.
//!
//! ## Serialization
//!
//! Enable the `serde` feature to derive `Serialize` and `Deserialize` on layout
//! types. Prefer declarative [`LayoutTree`] for workspace templates. Runtime [`Layout`] captures
//! split/tab state after user edits within the same application version; slotmap `NodeId` values
//! are not stable semantic handles across refactors.

// The ported sources keep upstream's lint allowances, which upstream declared in
// its own `Cargo.toml`. Reformatting the code to satisfy this crate's stricter
// pedantic rules would obscure the diff against upstream for no behavioural
// gain, so the same lints are allowed here instead.
#![allow(
    clippy::similar_names,
    clippy::trivially_copy_pass_by_ref,
    clippy::struct_excessive_bools,
    clippy::default_trait_access,
    clippy::implicit_hasher,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::uninlined_format_args,
    clippy::unused_self,
    clippy::type_complexity,
    clippy::doc_markdown,
    clippy::manual_assert_eq,
    clippy::manual_let_else
)]

pub mod builder;
pub mod error;
pub(crate) mod factory;
pub(crate) mod manager;
pub mod model;
pub mod panel;
pub mod persist;
pub mod prelude;
pub mod spatial;
pub mod style;
pub mod unstable;
pub mod widget;

pub use builder::{
    build_area, build_tree, horizontal, panel, panel_def, single, tabs, vertical, BuiltLayout,
    DockSession, DockSpec, InitialFocus, LayoutArea, LayoutTree, PaneTarget, PanelCycle, PanelDef,
    SplitNode, TabsNode,
};
pub use error::{Error, Result};
// `model::Dock` — one dock's open flag, collapsibility and size — is
// deliberately *not* re-exported at the root, and neither is
// `widget::Dock`. That name has meant "the whole dock widget" in every released
// version of this crate, so handing it back with a different meaning would be
// worse than requiring the qualified path `model::Dock` for the state struct.
pub use model::{DockPlacement, DockRegion, DockRegions, Layout};
pub use panel::{PanelControl, PanelPresentation, PanelStyle, PlainPanels};
pub use persist::{DockAreaState, DockSlot};
pub use spatial::{adjacent_pane, pane_bounds_map, Direction};
pub use style::{
    close_button_style, constant, default, preset, Catalog, CloseButtonStyle, DockBackgroundStyle,
    DockStyle, DropOverlayStyle, PaneContent, SplitterStyle, StyleFn, TabBarStyle, TabStyle,
    TabTooltipStyle, WindowStyle,
};
pub use widget::{
    dock, Dock, DockAction, DockEvent, DockWidgetState, TabAction, TabBarScrollbarAttachment,
};
