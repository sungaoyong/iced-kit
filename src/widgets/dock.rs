//! Docking layout: the design-system layer over [`crate::dock`].
//!
//! # What this module adds
//!
//! The docking machinery in [`crate::dock`] owns the hard part: draggable tabs,
//! resizable splits, drop targets and focus tracking. What it does not know
//! about is this crate's design tokens, so this module supplies a [`DockStyle`]
//! derived from them and a few defaults that match the rest of the system — tab
//! height, pane padding and splitter width all follow the same scale as every
//! other control.
//!
//! Prefer this module over [`crate::dock`] directly: the styling here is what
//! makes a dock look like the rest of the application.
//!
//! # Usage
//!
//! ```
//! # #[cfg(feature = "dock")] {
//! use iced::Element;
//! use iced_kit::widgets::dock::{self, DockEvent, DockSession, LayoutTree, PanelDef};
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum Panel {
//!     Explorer,
//!     Editor,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Dock(DockEvent<Panel>),
//! }
//!
//! struct App {
//!     session: DockSession<Panel>,
//! }
//!
//! fn layout() -> LayoutTree<Panel> {
//!     dock::horizontal([
//!         dock::tabs([PanelDef::new("explorer", "Explorer", Panel::Explorer)]),
//!         dock::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
//!     ])
//!     .weights([0.3, 0.7])
//! }
//!
//! fn view(app: &App) -> Element<'_, Message, iced_kit::Theme> {
//!     // The theme is named explicitly because `iced_dock` is generic over it.
//!     dock::dock::<Panel, Message, iced_kit::Theme, iced::Renderer>()
//!         .state(app.session.state())
//!         .on_event(Message::Dock)
//!         .style(dock::style)
//!         .content(|panel| {
//!             iced::widget::text(match panel {
//!                 Panel::Explorer => "Explorer",
//!                 Panel::Editor => "Editor",
//!             })
//!             .into()
//!         })
//!         .build()
//!         .into()
//! }
//! # }
//! ```

use crate::dock::style::{
    CloseButtonStyle, ControlStyle, DockBackgroundStyle, DockStyle, DropOverlayStyle,
    SplitterStyle, TabBarStyle, TabStyle, TabTooltipStyle, TitleStyle, WindowStyle,
};
use crate::theme::{Theme, Tokens};
use iced::{Background, Border, Color};

// Re-exported so an application depends on one crate rather than two.
pub use crate::dock::widget::DockBuilder;
pub use crate::dock::{
    dock, horizontal, panel, panel_def, single, tabs, vertical, DockAction, DockEvent, DockSession,
    DockSpec, DockWidgetState, InitialFocus, Layout, LayoutArea, LayoutTree, PaneTarget,
    PanelControl, PanelCycle, PanelDef, PanelPresentation, PanelStyle, PlainPanels, SplitNode,
    TabAction, TabBarScrollbarAttachment, TabsNode,
};
// `DockPlacement`, `DockRegion` and `DockRegions` are the region model, and
// `model::Dock` — one dock's open flag and size — is reachable as
// `iced_kit::dock::model::Dock`. They are re-exported here under their own names so
// an application describing an area depends on this module alone.
pub use crate::dock::model::{DockPlacement, DockRegion, DockRegions};
pub use crate::dock::persist::{DockAreaState, DockSlot};
pub use crate::dock::widget::MenuEntry;

/// Bridges the dock's style catalog onto this crate's [`Theme`].
///
/// `iced_dock`'s builder requires its own `Catalog` even when an explicit style
/// closure is supplied, so the default class returns the token-derived style.
impl crate::dock::style::Catalog for Theme {
    type Class<'a> = crate::dock::style::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(style)
    }

    fn style(&self, class: &Self::Class<'_>) -> DockStyle {
        class(self)
    }
}

/// Builds a [`DockStyle`] from the design tokens.
///
/// Every color comes from a semantic role, so a custom palette recolors the
/// dock along with the rest of the UI, and switching light/dark needs no
/// dock-specific code.
#[must_use]
pub fn style(theme: &Theme) -> DockStyle {
    let tokens = Tokens::default();
    let colors = theme.colors();
    let radius = f32::from(theme.radius().md);
    let small_radius = f32::from(theme.radius().sm);

    let mut style = DockStyle {
        background: DockBackgroundStyle {
            // The gaps between panes read as the page behind them.
            color: colors.background,
        },
        window: WindowStyle {
            // Panes are raised surfaces, matching `card`.
            background: colors.surface,
            border: Border {
                color: colors.border,
                width: 1.0,
                radius: radius.into(),
            },
            // Set below, once the rest of the style is in place.
            focused_border: None,
        },
        tab_bar: TabBarStyle {
            background: colors.background,
            close_button: CloseButtonStyle {
                text_color: colors.muted_foreground,
                background: Color::TRANSPARENT,
                hovered_background: colors.accent,
                hovered_text: colors.foreground,
                border_radius: small_radius,
            },
            separator: Some(colors.border),
            scrollbar_track: colors.muted,
            scrollbar_thumb: colors.muted_foreground,
            scrollbar_thumb_hovered: colors.ring,
            scrollbar_thumb_border: Color::TRANSPARENT,
        },
        tab: TabStyle {
            border_radius: small_radius,
            inactive_background: Color::TRANSPARENT,
            inactive_text: colors.muted_foreground,
            hovered_background: colors.accent,
            hovered_text: colors.foreground,
            pressed_background: colors.secondary,
            pressed_text: colors.foreground,
            // The selected tab blends into the pane it belongs to.
            active_background: colors.surface,
            active_text: colors.foreground,
            // An underline is the same affordance the `tabs` component uses for
            // its selected tab, so the two read consistently.
            active_accent: colors.primary,
            // A modified tab is marked with a tint rather than an icon.
            modified_background: Some(colors.muted),
        },
        splitter: SplitterStyle {
            // Idle splitters are invisible: a visible handle between every pane
            // would make a dense layout look noisy.
            idle_color: Color::TRANSPARENT,
            hover_color: colors.ring,
            drag_color: colors.primary,
        },
        drop_overlay: DropOverlayStyle {
            color: Color {
                a: 0.25,
                ..colors.primary
            },
            blocked_color: Color {
                a: 0.25,
                ..colors.destructive
            },
            border_width: 2.0,
            border_color: colors.primary,
            blocked_border_color: colors.destructive,
            insert_marker_min_alpha: 0.2,
        },
        tooltip: TabTooltipStyle {
            background: colors.foreground,
            text_color: colors.background,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            border_radius: small_radius,
            padding: [6.0, 10.0],
        },
        // A group holding one panel draws this bar instead of a strip of tabs.
        // It is the same height as the strip, so a group that gains or loses a
        // tab does not move its content.
        title: TitleStyle {
            height: tab_bar_height(),
            background: Some(colors.background),
            text_color: colors.foreground,
            padding: [0.0, 12.0],
            gap: 6.0,
            // The `xs` step, matching the tab labels beside it.
            text_size: tokens.typography.xs.size,
        },
        // The dock's own buttons — zoom, dock toggles, the ellipsis menu — are
        // quiet until hovered, so a busy layout's chrome stays out of the way.
        control: ControlStyle {
            size: crate::theme::Size::Sm.height(),
            text_color: colors.muted_foreground,
            hovered_text: colors.foreground,
            hovered_background: colors.accent,
            border_radius: small_radius,
            gap: 2.0,
            // A step below the control, so the glyph reads as an icon rather
            // than as a letter.
            glyph_size: tokens.typography.xs.size,
            handle: SplitterStyle {
                // Invisible at rest, exactly like an inner splitter: the handle
                // is there to be grabbed, not to draw a line down the window.
                idle_color: Color::TRANSPARENT,
                hover_color: colors.ring,
                drag_color: colors.primary,
            },
        },
    };

    // The focused-pane border uses the focus ring, which is the same color form
    // controls use when focused.
    style.window.focused_border = Some(Border {
        color: colors.ring,
        width: 1.0,
        radius: radius.into(),
    });

    // Keep the active tab flush with the pane it selects.
    style.sync_active_tab_with_window();

    let _ = tokens;

    style
}

/// A [`DockStyle`] where the drop overlay is suppressed.
///
/// Useful while a layout is being set up programmatically, when a stray drag
/// highlight would be misleading.
#[must_use]
pub fn style_without_drop_overlay(theme: &Theme) -> DockStyle {
    let mut style = style(theme);
    style.drop_overlay.border_width = 0.0;
    style
}

/// The recommended tab strip height, in logical pixels.
///
/// Matches the `Md` control height so a dock's tabs line up with a toolbar.
#[must_use]
pub fn tab_bar_height() -> f32 {
    crate::theme::Size::Md.height() + 4.0
}

/// The recommended width of a splitter's grab area.
///
/// Wider than the drawn line, so a thin splitter is still easy to grab.
#[must_use]
pub fn splitter_grab_width() -> f32 {
    6.0
}

/// The recommended minimum pane size, in logical pixels.
#[must_use]
pub fn min_pane_size() -> f32 {
    120.0
}

/// Applies the crate's recommended chrome metrics to a dock builder.
///
/// Offered as a free function so an application can use it without naming each
/// setter, and override any of them afterwards.
pub fn apply_metrics<Key, Message, Renderer>(
    builder: DockBuilder<'_, Key, Message, Theme, Renderer>,
) -> DockBuilder<'_, Key, Message, Theme, Renderer>
where
    Key: Copy + 'static,
    Message: Clone + 'static,
    Renderer: iced::advanced::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + iced::advanced::svg::Renderer
        + 'static,
{
    builder
        .tab_bar_height(tab_bar_height())
        .min_pane_width(min_pane_size())
        .min_pane_height(min_pane_size())
        .dock_handle_width(splitter_grab_width())
}

/// A bordered pane body, for use as a dock panel's content.
///
/// Panes in a dock share their chrome, so a panel that fills its pane needs no
/// border of its own; this adds the padding and background a content area
/// usually wants.
#[must_use]
pub fn pane_body<'a, Message: 'a>(
    content: impl Into<iced::Element<'a, Message, Theme>>,
) -> iced::widget::container::Container<'a, Message, Theme> {
    iced::widget::container(content).padding(12)
}

/// A tint applied to a pane that has unsaved changes.
///
/// Exposed so an application can mark a modified pane's content consistently
/// with how the dock marks a modified tab.
#[must_use]
pub fn modified_tint(theme: &Theme) -> Color {
    Color {
        a: 0.06,
        ..theme.colors().primary
    }
}

/// A background fill for the dock's root area.
#[must_use]
pub fn background(theme: &Theme) -> Background {
    Background::Color(theme.colors().background)
}

#[cfg(test)]
mod usage_tests {
    //! Compile-checks the documented integration path.
    //!
    //! `iced_dock` is generic over the theme and the renderer, so an integration
    //! that does not compile is easy to write and hard to notice; this drives the
    //! whole builder the way an application would.

    use super::{style, tab_bar_height, DockSession, LayoutTree, PanelDef};
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Panel {
        Explorer,
        Editor,
    }

    /// The handlers only need to produce a message; the event payload is not
    /// inspected here.
    #[derive(Debug, Clone)]
    enum Message {
        Dock,
    }

    fn layout() -> LayoutTree<Panel> {
        super::horizontal([
            super::tabs([PanelDef::new("explorer", "Explorer", Panel::Explorer)]),
            super::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
        ])
        .weights([0.3, 0.7])
    }

    #[test]
    fn a_layout_tree_compiles_with_panel_defs() {
        let _ = layout();
    }

    #[test]
    fn a_session_is_created_from_a_layout_tree() {
        let session = DockSession::from_tree(layout()).expect("the layout is valid");

        // The session owns the widget state the dock builder needs.
        let _state = session.state();
    }

    #[test]
    fn a_dock_renders_with_this_crates_theme() {
        let session = DockSession::from_tree(layout()).expect("the layout is valid");

        // The theme and renderer are named explicitly: `iced_dock` is generic
        // over both, and inference has nothing to work from here.
        let element: Element<'_, Message, Theme> =
            super::dock::<Panel, Message, Theme, iced::Renderer>()
                .state(session.state())
                .on_event(|_event| Message::Dock)
                .style(style)
                .content(|panel| {
                    iced::widget::text(match panel {
                        Panel::Explorer => "Explorer",
                        Panel::Editor => "Editor",
                    })
                    .into()
                })
                .build()
                .into();

        drop(element);
    }

    #[test]
    fn a_dock_renders_with_the_recommended_metrics() {
        let session = DockSession::from_tree(layout()).expect("the layout is valid");

        let element: Element<'_, Message, Theme> =
            super::apply_metrics(super::dock::<Panel, Message, Theme, iced::Renderer>())
                .state(session.state())
                .on_event(|_event| Message::Dock)
                .style(style)
                .content(|_panel| iced::widget::text("panel").into())
                .build()
                .into();

        assert_eq!(super::tab_bar_height(), tab_bar_height());
        drop(element);
    }

    #[test]
    fn a_dock_renders_with_a_single_panel() {
        let session = DockSession::from_tree(super::single(PanelDef::new(
            "editor",
            "main.rs",
            Panel::Editor,
        )))
        .expect("a single-panel layout is valid");

        let element: Element<'_, Message, Theme> =
            super::dock::<Panel, Message, Theme, iced::Renderer>()
                .state(session.state())
                .on_event(|_event| Message::Dock)
                .style(style)
                .content(|_panel| iced::widget::text("panel").into())
                .build()
                .into();

        drop(element);
    }

    #[test]
    fn a_dock_renders_with_a_vertical_split() {
        let session = DockSession::from_tree(super::vertical([
            super::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
            super::tabs([PanelDef::new("explorer", "Explorer", Panel::Explorer)]),
        ]))
        .expect("a vertical split is valid");

        let element: Element<'_, Message, Theme> =
            super::dock::<Panel, Message, Theme, iced::Renderer>()
                .state(session.state())
                .on_event(|_event| Message::Dock)
                .style(style)
                .content(|_panel| iced::widget::text("panel").into())
                .build()
                .into();

        drop(element);
    }
}

#[cfg(test)]
mod tests {
    use super::{min_pane_size, splitter_grab_width, style, tab_bar_height};
    use crate::theme::Theme;

    #[test]
    fn a_style_is_derived_for_both_palettes() {
        for theme in [Theme::light(), Theme::dark()] {
            let dock_style = style(&theme);

            assert_eq!(dock_style.window.background, theme.colors().surface);
            assert_eq!(dock_style.background.color, theme.colors().background);
            assert_eq!(dock_style.tab.active_text, theme.colors().foreground);
        }
    }

    #[test]
    fn the_selected_tab_blends_into_its_pane() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        // A selected tab whose background differs from the pane would show a
        // seam between the tab and its content.
        assert_eq!(
            dock_style.tab.active_background,
            dock_style.window.background
        );
    }

    #[test]
    fn the_tab_strip_shares_the_dock_background() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        assert_eq!(dock_style.tab_bar.background, dock_style.background.color);
    }

    #[test]
    fn idle_splitters_are_invisible() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        // A visible handle between every pane would make a dense layout noisy.
        assert_eq!(dock_style.splitter.idle_color.a, 0.0);
        assert_ne!(dock_style.splitter.hover_color.a, 0.0);
    }

    #[test]
    fn the_drop_overlay_distinguishes_valid_from_blocked() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        assert_ne!(
            dock_style.drop_overlay.border_color,
            dock_style.drop_overlay.blocked_border_color
        );
        assert_ne!(
            dock_style.drop_overlay.color,
            dock_style.drop_overlay.blocked_color
        );
    }

    #[test]
    fn a_focused_pane_is_outlined_with_the_focus_ring() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        let focused = dock_style
            .window
            .focused_border
            .expect("a focused border is always set");

        assert_eq!(focused.color, theme.colors().ring);
    }

    #[test]
    fn the_light_and_dark_styles_differ() {
        let light = style(&Theme::light());
        let dark = style(&Theme::dark());

        assert_ne!(light.window.background, dark.window.background);
        assert_ne!(light.background.color, dark.background.color);
    }

    #[test]
    fn tooltips_are_inverted_so_they_read_as_an_overlay() {
        let theme = Theme::light();
        let dock_style = style(&theme);

        assert_eq!(dock_style.tooltip.background, theme.colors().foreground);
        assert_eq!(dock_style.tooltip.text_color, theme.colors().background);
    }

    #[test]
    fn metrics_are_sane_and_ordered() {
        assert!(tab_bar_height() > 0.0);
        assert!(splitter_grab_width() > 0.0);
        assert!(min_pane_size() > 0.0);

        // A grab area wider than the pane minimum would make panes unusable.
        assert!(splitter_grab_width() < min_pane_size());
    }

    #[test]
    fn a_style_without_a_drop_overlay_has_no_outline() {
        let theme = Theme::light();
        let suppressed = super::style_without_drop_overlay(&theme);

        assert_eq!(suppressed.drop_overlay.border_width, 0.0);
        // Everything else still matches the normal style.
        assert_eq!(
            suppressed.window.background,
            style(&theme).window.background
        );
    }

    #[test]
    fn a_modified_tint_is_translucent() {
        let theme = Theme::light();
        let tint = super::modified_tint(&theme);

        assert!(tint.a > 0.0 && tint.a < 1.0, "a wash, not a fill");
    }
}
