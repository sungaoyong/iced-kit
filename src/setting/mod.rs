//! Application settings panels.
//!
//! An application's settings are a tree: pages hold groups, groups hold items,
//! and an item is a labelled control bound to one value. This module builds that
//! tree and draws it with a search box, a navigable sidebar and per-page reset.
//!
//! # The shape of a settings panel
//!
//! ```text
//! Settings
//!   SettingPage        <- the sidebar lists these, and one is shown at a time
//!     SettingGroup     <- a titled box; the sidebar can list these too
//!       SettingItem    <- a label, a description, and one field
//! ```
//!
//! # Where the state lives
//!
//! The caller owns the values, as everywhere else in iced. A field is built from
//! the current value plus a closure that turns the new value into the caller's
//! `Message`:
//!
//! ```
//! use iced_kit::setting::{
//!     SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
//! };
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! #[derive(Clone, Debug)]
//! enum Message {
//!     AutoUpdate(bool),
//!     FontSize(f64),
//! }
//!
//! struct App {
//!     settings: SettingsState,
//!     auto_update: bool,
//!     font_size: f64,
//! }
//!
//! fn view(app: &App) -> Element<'_, Message, Theme> {
//!     Settings::new(&app.settings)
//!         .page(
//!             SettingPage::new("General").group(
//!                 SettingGroup::new()
//!                     .title("Updates")
//!                     .item(
//!                         SettingItem::new("Automatic updates")
//!                             .description("Install new versions in the background.")
//!                             .field(SettingField::switch(
//!                                 app.auto_update,
//!                                 Message::AutoUpdate,
//!                             )),
//!                     )
//!                     .item(
//!                         SettingItem::new("Font size")
//!                             .field(SettingField::number(
//!                                 app.font_size,
//!                                 8.0..=72.0,
//!                                 Message::FontSize,
//!                             )),
//!                     ),
//!             ),
//!         )
//!         .into()
//! }
//! ```
//!
//! # What the panel owns, and what it asks you for
//!
//! The panel owns its *view* state: which page is selected, the search query,
//! which groups have been expanded, and where the content is scrolled. That is
//! state about the panel rather than about the settings, so keeping it inside
//! means an application does not have to thread it through its own `Message`.
//!
//! The panel asks you for the *values*, and for what a reset means. A field with
//! no [`default_value`](SettingField::default_value) cannot be reset, which is
//! how the reset button knows whether to appear at all.
//!
//! # Searching
//!
//! A query filters items by title, description and any explicit
//! [`keywords`](SettingItem::keywords). Groups and pages with no match drop out,
//! and a page whose every group was filtered away is hidden from the sidebar —
//! so an unmatched query leaves nothing to click, which is what a search is for.
//!
//! # What iced does not have here
//!
//! - **A resizable sidebar.** iced's `pane_grid` splits proportionally, so the
//!   sidebar has a fixed width rather than a pixel range the user can drag
//!   within. See [`Settings::sidebar_width`].
//! - **Scroll-to-group.** Selecting a group in the sidebar scrolls the content
//!   column to it, which needs each group's offset. That is measured from the
//!   layout rather than the model, so it takes effect on the frame after the
//!   page is drawn.
//! - **Per-item styling.** The reference lets a caller refine the style of any
//!   part; here a field is an `Element`, so style it before handing it over.

mod field;
mod group;
mod item;
mod page;
mod panel;
mod scroll;
mod search;

pub use field::{SettingField, SettingFieldKind, SettingValue};
pub use group::SettingGroup;
pub use item::{SettingItem, SettingLayout};
pub use page::SettingPage;
pub use panel::{
    content_id, group_id, settings, Settings, SettingsEvent, SettingsState,
    STACKED_LAYOUT_MAX_WIDTH, STACKED_SIDEBAR_MAX_HEIGHT,
};
pub use search::matches;
