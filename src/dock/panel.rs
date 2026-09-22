// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! What a panel adds to its own chrome: a title, a toolbar, a menu.
//!
//! The dock owns the layout of a tab bar, but it cannot know what a panel wants
//! to put in one. This module is that seam: a [`PanelPresentation`] the
//! application supplies, consulted while the tab bar is built, so a panel can
//! draw a title with an icon, pin buttons to the trailing end, and add entries to
//! the tab bar's menu — without the dock knowing what any of them mean.
//!
//! Every method has a default that draws nothing, so a panel that wants no chrome
//! does not implement any of them.

use iced::Element;

use crate::dock::style::Catalog;

/// The panel chrome the dock asks for while building a tab bar.
///
/// Implemented by the application for its panel key type `K`. Keys are cheap
/// copies, so the trait is queried per panel per frame — the same shape as the
/// existing `content` and `modified` closures — and there is no panel state to
/// keep in step with the layout.
///
/// ```ignore
/// struct Panels;
///
/// impl PanelPresentation<Panel, Message> for Panels {
///     fn title(&self, panel: Panel) -> Option<Element<'_, Message>> {
///         Some(row![icon(panel.icon()), text(panel.title())].into())
///     }
///
///     fn toolbar(&self, panel: Panel) -> Vec<Element<'_, Message>> {
///         vec![icon_button("save").into()]
///     }
/// }
///
/// dock()
///     .presentation(Panels)
///     ...
/// ```
///
/// # Lifetimes
///
/// The methods return `Element<'static, Message, Theme, Renderer>` borrowing `&self`,
/// so an implementation may hand back a borrowed label without cloning. The dock
/// only keeps the element for the frame it built it in, which is what makes that
/// sound.
pub trait PanelPresentation<K, Message, Theme = crate::theme::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    /// The element drawn in a tab, or in a single-panel title bar.
    ///
    /// `None` falls back to the panel's title string from its
    /// [`PanelDef`](crate::dock::PanelDef), so a panel that has nothing special
    /// to draw needs no implementation.
    fn title(&self, _panel: K) -> Option<Element<'static, Message, Theme, Renderer>> {
        None
    }

    /// An element pinned to the trailing end of the title bar, before the
    /// toolbar.
    fn title_suffix(&self, _panel: K) -> Option<Element<'static, Message, Theme, Renderer>> {
        None
    }

    /// Buttons for the title bar's trailing toolbar.
    ///
    /// Each is truncated to a square of the tab bar's height, so a row of them
    /// reads as one control group regardless of how the application built them.
    /// Zoom and the ellipsis menu follow, so the group ends in a predictable
    /// place.
    fn toolbar(&self, _panel: K) -> Vec<Element<'static, Message, Theme, Renderer>> {
        Vec::new()
    }

    /// Entries the panel adds to the tab bar's ellipsis menu.
    ///
    /// The dock adds the zoom and close entries after these, separated, so a
    /// panel never has to implement either.
    fn menu(&self, _panel: K) -> Vec<(String, Message)> {
        Vec::new()
    }

    /// Where this panel's zoom control appears.
    ///
    /// Default [`PanelControl::Menu`]. `None` withholds the affordance
    /// altogether: a panel that offers no control is not zoomed even by the
    /// keyboard, so "never zoom" is stated once.
    fn zoom_control(&self, _panel: K) -> Option<PanelControl> {
        Some(PanelControl::Menu)
    }

    /// Whether the tab group pads the panel's content.
    ///
    /// `false` for content that draws its own edges — a code editor, an image —
    /// and wants the full pane. Default `true`.
    fn inner_padding(&self, _panel: K) -> bool {
        true
    }

    /// Whether a single-panel group draws a title bar above the content.
    ///
    /// `false` for a panel that carries its own chrome. A group holding several
    /// panels still draws its tab strip, so every panel stays reachable.
    /// Default `true`.
    fn title_bar(&self, _panel: K) -> bool {
        true
    }
}

/// A [`PanelPresentation`] that has nothing to say, for a dock whose panels are
/// plain.
///
/// Every method at its default. Named rather than left to `()` so the intent
/// reads at the call site.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainPanels;

impl<K, Message, Theme, Renderer> PanelPresentation<K, Message, Theme, Renderer> for PlainPanels where
    Theme: Catalog
{
}

/// Where a panel's zoom control appears.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PanelControl {
    /// Both the title bar and the menu.
    Both,
    /// Only the ellipsis menu. The default, and what a title bar with no room
    /// wants.
    #[default]
    Menu,
    /// Only the title bar.
    Toolbar,
}

impl PanelControl {
    #[must_use]
    pub fn toolbar_visible(self) -> bool {
        matches!(self, Self::Both | Self::Toolbar)
    }

    #[must_use]
    pub fn menu_visible(self) -> bool {
        matches!(self, Self::Both | Self::Menu)
    }
}

/// How a tab group presents itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PanelStyle {
    /// A single panel draws a plain title bar; two or more draw a full tab strip.
    ///
    /// This is the default, and what an editor wants: one document needs no tab
    /// strip, several do.
    #[default]
    Auto,
    /// Always draw the tab strip, even for a single panel.
    TabBar,
}
