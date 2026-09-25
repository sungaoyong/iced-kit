//! Ribbon — a tabbed toolbar of grouped, multi-size tool buttons.
//!
//! A ribbon puts a command palette in two bands: a strip of tabs along the top,
//! and beneath the active tab a row of *groups*, each a boxed cluster of tool
//! buttons with a small label along the bottom edge. Tools come in a large
//! (full-height, icon over label) and a small (single-row, icon only or icon
//! beside label) footprint, and either can carry a ▾ that opens a dropdown of
//! related commands.
//!
//! # It is data, and the application owns the state
//!
//! A ribbon is described entirely as data — [`RibbonTab`]s of [`RibbonGroup`]s
//! of [`RibbonItem`]s wrapping [`RibbonTool`]s — and rebuilt every frame, as
//! every iced-kit component is. Which tab is active and which dropdown is open
//! live in a [`RibbonState`] the caller holds; the ribbon reports intent through
//! [`on_select`](Ribbon::on_select) and
//! [`on_dropdown_toggle`](Ribbon::on_dropdown_toggle) and never mutates anything
//! itself.
//!
//! # Dropdowns are hosted, not painted
//!
//! iced has no window-level z-order, so a ribbon draws no floating panel of its
//! own. A ▾ reports the id it wants opened; the application builds that panel —
//! from the [`RibbonItem::dropdown`] payload it already owns — and hosts it
//! through [`Layer`](crate::widgets::overlay::Layer), anchored with
//! [`trigger`](crate::widgets::overlay::trigger), exactly as it does for the
//! combobox, date-picker and colour-picker panels.
//!
//! # Layout
//!
//! Within a group, a large item owns a full-height column while small items
//! stack three to a column, matching the reference's packing. When the groups
//! outrun the window the band degrades *from the right*, one group at a time —
//! full to compact icon columns, then to a title button, then to a tight
//! small-icon button — and the row's height shrinks with it. That is
//! [`CollapseMode::Auto`](collapse::CollapseMode::Auto); the other modes pin
//! every group to one density. See [`collapse`](super::collapse) for the ladder.
//!
//! ```
//! use iced_kit::widgets::ribbon::{Ribbon, RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! #[derive(Clone, Debug)]
//! enum Message { Selected(usize) }
//!
//! fn view(state: &RibbonState) -> Element<'static, Message, Theme> {
//!     Ribbon::new()
//!         .tab(RibbonTab::new("Home").group(
//!             RibbonGroup::new("Draw")
//!                 .item(RibbonItem::large(RibbonTool::new("／").label("Line")))
//!                 .item(RibbonItem::tool(RibbonTool::new("▢").label("Rectangle"))),
//!         ))
//!         .state(state)
//!         .on_select(Message::Selected)
//!         .into()
//! }
//! ```

mod builder;
mod buttons;
mod collapse;
mod model;
mod style;

pub use builder::{ribbon, Ribbon};
pub use collapse::CollapseMode;
pub use model::{RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool};
