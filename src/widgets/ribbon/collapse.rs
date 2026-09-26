//! Adaptive ribbon density and the reference's six panel layouts.
//!
//! A ribbon lays its groups on one row. With [`RibbonLayout::Auto`], when the
//! groups no longer all fit, the row degrades *from the right*, one group at a
//! time: first a group shrinks to a compact column of small icons, then it
//! collapses to a title button. If even the all-collapsed row overflows, every
//! collapsed group drops to a tight small-icon button together, and the buttons
//! are then squeezed. The row's height tracks the tallest group still shown, so
//! it shrinks as the groups do.
//!
//! A fixed [`RibbonLayout`] — the reference's loose/compact × three/two/single
//! row styles — pins every group to its own rendering instead, so a two-row or
//! single-row group holds its shape rather than degrading.
//!
//! The decision is pure ([`decide_levels`]) and unit-tested; [`CollapseGroups`]
//! is the thin iced widget that measures each group at all four densities, runs
//! the decision against the width it was offered, and lays out the chosen
//! rendering of each. Unlike the reference, it reports nothing through atomics:
//! a collapsed group is simply a button that toggles a dropdown, so its panel
//! is hosted by the application through [`Layer`](crate::widgets::overlay::Layer)
//! like every other ribbon dropdown, and the widget forwards its children's own
//! overlays (tooltips) untouched.

use super::model::RibbonLayout;
use super::style::TOOL_BAR_H;
use crate::theme::Theme;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};
use std::cell::RefCell;

/// Renderings held per group: full, compact, collapsed, tight.
const SLOTS: usize = 4;

/// When even the all-tight row still overflows, the collapsed buttons are
/// pulled together by up to this many pixels per gap — reclaiming their edge
/// padding — before anything is clipped.
const MAX_GROUP_SQUEEZE: f32 = 8.0;

/// How the ribbon's groups are sized.
///
/// [`Auto`](Self::Auto) adapts to the offered width, degrading groups from the
/// right as space runs out; the others pin every group to one density so a user
/// can override the automatic choice. It is plain data, so an application can
/// persist it alongside the rest of its [`RibbonState`](super::RibbonState).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CollapseMode {
    /// Size the groups to the window: degrade from the right as space runs out.
    #[default]
    Auto,
    /// Always full-size groups, even if they overflow.
    Full,
    /// Always compact groups (small icon columns).
    Compact,
    /// Always collapsed to title buttons.
    Collapsed,
}

impl CollapseMode {
    /// Every mode, in the order a selector would list them.
    pub const ALL: &'static [CollapseMode] =
        &[Self::Auto, Self::Full, Self::Compact, Self::Collapsed];

    /// The label for a mode, for a settings control.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Full => "Full",
            Self::Compact => "Compact",
            Self::Collapsed => "Collapsed",
        }
    }
}

impl std::fmt::Display for CollapseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A group's degradation level; also its offset among the group's tree slots.
/// The degrade order, from the right, is
/// `Full` → `Compact` → `Collapsed` (title button) → `Tight` (small-icon button).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Full = 0,
    Compact = 1,
    Collapsed = 2,
    Tight = 3,
}

/// Tree-slot index of group `i`'s rendering at `level`.
fn slot(i: usize, level: Level) -> usize {
    i * SLOTS + level as usize
}

/// A group's natural width at each of the four densities. Only widths drive the
/// degradation decision; the decision itself is pure and tested.
#[derive(Debug, Clone, Copy, Default)]
struct Widths {
    full: f32,
    compact: f32,
    button: f32,
    tight: f32,
}

/// Choose a level per group. [`RibbonLayout::Auto`] degrades from the right,
/// one group at a time: first Full → Compact, then Compact → Collapsed, each
/// phase only while the row still overflows; if even the all-collapsed row
/// overflows, every collapsed group drops to Tight at once. A fixed
/// [`RibbonLayout`] pins every group to the row style's own rendering, so the
/// two-row and single-row layouts hold their shape instead of degrading.
fn decide_levels(layout: RibbonLayout, widths: &[Widths], max_w: f32) -> Vec<Level> {
    if layout != RibbonLayout::Auto {
        return vec![Level::Full; widths.len()];
    }
    let width_of = |lv: Level, i: usize| match lv {
        Level::Full => widths[i].full,
        Level::Compact => widths[i].compact,
        Level::Collapsed => widths[i].button,
        Level::Tight => widths[i].tight,
    };
    let total =
        |levels: &[Level]| -> f32 { (0..levels.len()).map(|i| width_of(levels[i], i)).sum() };

    let mut levels = vec![Level::Full; widths.len()];
    for degraded in [Level::Compact, Level::Collapsed] {
        for i in (0..widths.len()).rev() {
            if total(&levels) <= max_w {
                break;
            }
            levels[i] = degraded;
        }
    }
    if total(&levels) > max_w {
        levels
            .iter_mut()
            .filter(|l| **l == Level::Collapsed)
            .for_each(|l| *l = Level::Tight);
    }
    levels
}

/// The four renderings of one group, in `Level` order:
/// `[full, compact, collapsed, tight]`.
pub(crate) type GroupSlots<'a, Message> = [Element<'a, Message, Theme>; SLOTS];

/// Lays a row of groups out at their chosen densities.
///
/// Construct it from one [`GroupSlots`] per group; it owns the elements and, on
/// each layout, decides how far to degrade them against the width it is offered.
pub(crate) struct CollapseGroups<'a, Message> {
    /// The groups' renderings, flattened in slot order (`slot(i, level)`).
    elements: Vec<Element<'a, Message, Theme>>,
    /// How many groups are laid out; `elements.len() == count * SLOTS`.
    count: usize,
    layout: RibbonLayout,
    /// The levels chosen during the last layout, replayed in the other trait
    /// methods so update/draw/overlay touch the same child that was placed.
    levels: RefCell<Vec<Level>>,
}

impl<'a, Message: Clone + 'a> CollapseGroups<'a, Message> {
    /// Builds the row from one [`GroupSlots`] per group, in left-to-right order.
    pub(crate) fn new(groups: Vec<GroupSlots<'a, Message>>, layout: RibbonLayout) -> Self {
        let count = groups.len();
        let elements = groups.into_iter().flatten().collect();
        Self {
            elements,
            count,
            layout,
            levels: RefCell::new(vec![Level::Full; count]),
        }
    }

    /// The chosen level of group `i`, defaulting to `Full` before a layout.
    fn level(&self, i: usize) -> Level {
        self.levels.borrow().get(i).copied().unwrap_or(Level::Full)
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for CollapseGroups<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.elements.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.elements);
    }

    fn size(&self) -> Size<Length> {
        // Fill so `layout` is offered the window width to degrade against;
        // Shrink height so the row is as tall as its tallest shown group.
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if self.count == 0 {
            return layout::Node::new(Size::new(0.0, TOOL_BAR_H));
        }

        let natural = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY));
        let auto = self.layout == RibbonLayout::Auto;

        // Auto needs each group's width at all four densities to pick the
        // degradation. A fixed layout pins every group to its own rendering,
        // so it measures nothing up front and derives the row from what it
        // places.
        let widths = if auto {
            let mut measured = Vec::with_capacity(self.count);
            for i in 0..self.count {
                measured.push(Widths {
                    full: measure(self, tree, renderer, &natural, slot(i, Level::Full)),
                    compact: measure(self, tree, renderer, &natural, slot(i, Level::Compact)),
                    button: measure(self, tree, renderer, &natural, slot(i, Level::Collapsed)),
                    tight: measure(self, tree, renderer, &natural, slot(i, Level::Tight)),
                });
            }
            measured
        } else {
            // A fixed layout ignores the widths but still needs one level per
            // group, so hand `decide_levels` a correctly-sized placeholder.
            vec![Widths::default(); self.count]
        };

        let levels = decide_levels(self.layout, &widths, limits.max().width);
        *self.levels.borrow_mut() = levels;
        let levels = self.levels.borrow();

        // Place the chosen rendering of each group and note its size.
        let mut placed: Vec<(layout::Node, f32, f32)> = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let level = levels[i];
            let node = self.elements[slot(i, level)].as_widget_mut().layout(
                &mut tree.children[slot(i, level)],
                renderer,
                &natural,
            );
            let size = node.size();
            placed.push((node, size.width, size.height));
        }

        // The row is as tall as the tallest shown group, so it shrinks as the
        // groups degrade to shorter collapsed / tight buttons (each rendering
        // carries its own height).
        let row_h = placed.iter().map(|&(_, _, h)| h).fold(0.0f32, f32::max);

        // Once every group is at its tightest and the row STILL overflows, pull
        // the buttons together (up to the cap per gap) so more stay on screen
        // before anything is clipped. In Auto this only fires when everything is
        // already tight; a forced density is left to overflow.
        let total: f32 = placed.iter().map(|&(_, w, _)| w).sum();
        let squeeze = if auto && self.count > 1 && total > limits.max().width {
            ((total - limits.max().width) / (self.count - 1) as f32).min(MAX_GROUP_SQUEEZE)
        } else {
            0.0
        };

        let mut x = 0.0f32;
        let children = placed
            .into_iter()
            .map(|(node, w, h)| {
                if x > 0.0 {
                    x -= squeeze;
                }
                let y = ((row_h - h) / 2.0).max(0.0);
                let positioned = node.move_to(Point::new(x, y));
                x += w;
                positioned
            })
            .collect();

        layout::Node::with_children(Size::new(x, row_h), children)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for (i, child) in layout.children().enumerate() {
            let s = slot(i, self.level(i));
            self.elements[s].as_widget_mut().update(
                &mut tree.children[s],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut interaction = mouse::Interaction::None;
        for (i, child) in layout.children().enumerate() {
            let s = slot(i, self.level(i));
            let it = self.elements[s].as_widget().mouse_interaction(
                &tree.children[s],
                child,
                cursor,
                viewport,
                renderer,
            );
            if it != mouse::Interaction::None {
                interaction = it;
            }
        }
        interaction
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for (i, child) in layout.children().enumerate() {
            let s = slot(i, self.level(i));
            self.elements[s].as_widget_mut().operate(
                &mut tree.children[s],
                child,
                renderer,
                operation,
            );
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for (i, child) in layout.children().enumerate() {
            let s = slot(i, self.level(i));
            self.elements[s].as_widget().draw(
                &tree.children[s],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        // No internal flyout — a collapsed group's panel is hosted by the
        // application. But each shown child may carry its own overlay (a
        // tooltip), so forward those rather than dropping them on the floor.
        // `iter_mut` keeps each child's borrow disjoint so several can be raised
        // at once; `levels` and the per-group layout nodes are read up front, so
        // nothing borrows `self` immutably during the walk.
        let levels = self.levels.borrow().clone();
        let layouts: Vec<Layout<'b>> = layout.children().collect();
        let mut overlays = Vec::new();
        for (s, (element, child)) in self
            .elements
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            let group = s / SLOTS;
            if group >= layouts.len() || levels[group] as usize != s % SLOTS {
                continue;
            }
            if let Some(overlay) = element.as_widget_mut().overlay(
                child,
                layouts[group],
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }
        if overlays.is_empty() {
            None
        } else {
            Some(overlay::Group::with_children(overlays).overlay())
        }
    }
}

/// Measures one child slot at its natural (unbounded) width.
fn measure<Message: Clone>(
    this: &mut CollapseGroups<'_, Message>,
    tree: &mut Tree,
    renderer: &iced::Renderer,
    limits: &layout::Limits,
    s: usize,
) -> f32 {
    this.elements[s]
        .as_widget_mut()
        .layout(&mut tree.children[s], renderer, limits)
        .size()
        .width
}

impl<'a, Message: Clone + 'a> From<CollapseGroups<'a, Message>> for Element<'a, Message, Theme> {
    fn from(groups: CollapseGroups<'a, Message>) -> Self {
        Element::new(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::{decide_levels, Level, RibbonLayout, Widths};

    fn w(full: f32, compact: f32, button: f32, tight: f32) -> Widths {
        Widths {
            full,
            compact,
            button,
            tight,
        }
    }

    #[test]
    fn fixed_layouts_pin_every_group() {
        let widths = [w(200.0, 150.0, 100.0, 50.0), w(180.0, 130.0, 90.0, 40.0)];
        for layout in [
            RibbonLayout::LooseThreeRow,
            RibbonLayout::CompactThreeRow,
            RibbonLayout::LooseTwoRow,
            RibbonLayout::CompactTwoRow,
            RibbonLayout::LooseSingleRow,
            RibbonLayout::CompactSingleRow,
        ] {
            assert_eq!(
                decide_levels(layout, &widths, 10.0),
                vec![Level::Full, Level::Full],
                "{layout:?} must pin every group to its own rendering"
            );
        }
    }

    #[test]
    fn auto_huge_width_keeps_everything_full() {
        let widths = [w(200.0, 150.0, 100.0, 50.0), w(180.0, 130.0, 90.0, 40.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 1e9),
            vec![Level::Full, Level::Full]
        );
    }

    #[test]
    fn auto_degrades_from_the_right_one_group_at_a_time() {
        let widths = [w(50.0, 40.0, 30.0, 20.0), w(200.0, 100.0, 60.0, 30.0)];
        // Full row = 250 > 240; rightmost compact => 50 + 100 = 150 <= 240.
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 240.0),
            vec![Level::Full, Level::Compact]
        );
    }

    #[test]
    fn auto_stops_degrading_once_the_row_fits() {
        let widths = [w(50.0, 25.0, 20.0, 15.0), w(50.0, 25.0, 20.0, 15.0)];
        // Full row 100 > 75; rightmost compact => 75 <= 75; left stays FULL.
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 75.0),
            vec![Level::Full, Level::Compact]
        );
    }

    #[test]
    fn auto_escalates_a_single_group_past_collapsed_to_tight() {
        let widths = [w(50.0, 40.0, 30.0, 20.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 25.0),
            vec![Level::Tight]
        );
    }

    #[test]
    fn auto_cascade_drops_every_collapsed_group_to_tight() {
        let widths = [w(100.0, 90.0, 50.0, 30.0), w(100.0, 90.0, 50.0, 30.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 20.0),
            vec![Level::Tight, Level::Tight]
        );
    }

    #[test]
    fn auto_runs_the_compact_phase_across_the_row_first() {
        let widths = [w(30.0, 26.0, 15.0, 12.0), w(100.0, 80.0, 40.0, 20.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 100.0),
            vec![Level::Compact, Level::Collapsed]
        );
    }

    #[test]
    fn no_groups_stays_empty() {
        assert_eq!(decide_levels(RibbonLayout::Auto, &[], 1e9), vec![]);
        assert_eq!(decide_levels(RibbonLayout::LooseThreeRow, &[], 1e9), vec![]);
    }

    #[test]
    fn single_group_walks_the_whole_ladder() {
        let widths = [w(100.0, 60.0, 40.0, 20.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 1e9),
            vec![Level::Full]
        );
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 75.0),
            vec![Level::Compact]
        );
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 45.0),
            vec![Level::Collapsed]
        );
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 20.0),
            vec![Level::Tight]
        );
    }

    #[test]
    fn exact_fit_at_full_stays_full() {
        let widths = [w(30.0, 20.0, 15.0, 10.0), w(40.0, 25.0, 15.0, 10.0)];
        assert_eq!(
            decide_levels(RibbonLayout::Auto, &widths, 70.0),
            vec![Level::Full, Level::Full]
        );
    }
}
