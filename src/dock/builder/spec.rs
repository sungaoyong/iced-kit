// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::collections::HashSet;

use crate::dock::model::Axis;
use crate::dock::Error;

/// Declarative layout description.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LayoutTree<K> {
    /// Tabbed pane (typical split leaf).
    Tabs(TabsNode<K>),
    /// Nested split container.
    Split(SplitNode<K>),
}

/// Panel metadata used when building or opening tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PanelDef<K> {
    pub id: String,
    pub title: String,
    pub content: K,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
    /// Whether the zoom affordance may maximize this panel.
    pub can_zoom: bool,
    /// Whether the tab bar offers this panel. A hidden panel keeps its place.
    pub visible: bool,
    /// A short name for an already-collapsed tab group, when the full title will
    /// not fit. `None` falls back to the title.
    pub tab_name: Option<String>,
    pub group: Option<String>,
}

impl<K: Copy> PanelDef<K> {
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: K) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content,
            can_close: true,
            can_drag: true,
            can_drop: true,
            can_zoom: true,
            visible: true,
            tab_name: None,
            group: None,
        }
    }

    #[must_use]
    pub fn can_close(mut self, value: bool) -> Self {
        self.can_close = value;
        self
    }

    #[must_use]
    pub fn can_drag(mut self, value: bool) -> Self {
        self.can_drag = value;
        self
    }

    #[must_use]
    pub fn can_drop(mut self, value: bool) -> Self {
        self.can_drop = value;
        self
    }

    /// Whether the zoom affordance may maximize this panel. Default `true`.
    #[must_use]
    pub fn can_zoom(mut self, value: bool) -> Self {
        self.can_zoom = value;
        self
    }

    /// Whether the tab bar offers this panel. Default `true`.
    ///
    /// A hidden panel stays in its pane and keeps its tab position, so showing
    /// it again restores the layout; it is left out of the strip meanwhile.
    #[must_use]
    pub fn visible(mut self, value: bool) -> Self {
        self.visible = value;
        self
    }

    /// A short name used when the tab bar has no room for the full title.
    #[must_use]
    pub fn tab_name(mut self, name: impl Into<String>) -> Self {
        self.tab_name = Some(name.into());
        self
    }

    #[must_use]
    pub fn group(mut self, g: impl Into<String>) -> Self {
        self.group = Some(g.into());
        self
    }
}

/// Tabbed pane node.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TabsNode<K> {
    pub name: Option<String>,
    pub panels: Vec<PanelDef<K>>,
    pub active: Option<String>,
    pub group: Option<String>,
    pub persistent: bool,
}

impl<K: Copy> TabsNode<K> {
    pub fn new(panels: impl IntoIterator<Item = PanelDef<K>>) -> Self {
        Self {
            name: None,
            panels: panels.into_iter().collect(),
            active: None,
            group: None,
            persistent: false,
        }
    }

    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn active(mut self, panel_id: impl Into<String>) -> Self {
        self.active = Some(panel_id.into());
        self
    }

    #[must_use]
    pub fn group(mut self, g: impl Into<String>) -> Self {
        self.group = Some(g.into());
        self
    }

    #[must_use]
    pub fn persistent(mut self, value: bool) -> Self {
        self.persistent = value;
        self
    }
}

/// Split container node.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitNode<K> {
    pub axis: Axis,
    pub children: Vec<LayoutTree<K>>,
    pub weights: Option<Vec<f32>>,
}

impl<K> SplitNode<K> {
    pub fn new(axis: Axis, children: impl IntoIterator<Item = LayoutTree<K>>) -> Self {
        Self {
            axis,
            children: children.into_iter().collect(),
            weights: None,
        }
    }

    #[must_use]
    pub fn weights(mut self, weights: impl IntoIterator<Item = f32>) -> Self {
        self.weights = Some(weights.into_iter().collect());
        self
    }
}

impl<K: Copy> LayoutTree<K> {
    /// Set the active tab on a `Tabs` node.
    #[must_use]
    pub fn active(mut self, panel_id: impl Into<String>) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.active = Some(panel_id.into());
        }
        self
    }

    /// Assign a stable name to a `Tabs` node for [`PaneTarget::Named`](crate::dock::builder::PaneTarget).
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.name = Some(name.into());
        }
        self
    }

    /// Set split weights on a `Split` node.
    #[must_use]
    pub fn weights(mut self, weights: impl IntoIterator<Item = f32>) -> Self {
        if let Self::Split(ref mut node) = self {
            node.weights = Some(weights.into_iter().collect());
        }
        self
    }

    /// Assign a tab group to a `Tabs` node.
    #[must_use]
    pub fn group(mut self, g: impl Into<String>) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.group = Some(g.into());
        }
        self
    }

    /// Mark a `Tabs` node as persistent (never collapsed when empty).
    #[must_use]
    pub fn persistent(mut self, value: bool) -> Self {
        if let Self::Tabs(ref mut node) = self {
            node.persistent = value;
        }
        self
    }
}

/// Create a panel definition (for use inside [`tabs`]).
pub fn panel<K: Copy>(id: impl Into<String>, title: impl Into<String>, content: K) -> PanelDef<K> {
    PanelDef::new(id, title, content)
}

/// A whole window layout: a centre tree plus any edge docks.
///
/// This is what a workspace template describes, as opposed to a single
/// [`LayoutTree`], which is one region's contents. Compile it with
/// [`build_area`](crate::dock::builder::build_area) to get the regions a
/// [`DockSession`](crate::dock::DockSession) installs.
///
/// ```ignore
/// let area = LayoutArea::new(single(PanelDef::new("editor", "main.rs", Panel::Editor)))
///     .dock(DockPlacement::Left, 240.0, tabs([PanelDef::new("files", "Files", Panel::Files)]))
///     .dock(DockPlacement::Bottom, 200.0, tabs([PanelDef::new("term", "Terminal", Panel::Terminal)]));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayoutArea<K> {
    /// The centre tree.
    pub center: LayoutTree<K>,
    /// Each edge dock that should exist, with its initial size and whether it
    /// starts open.
    pub docks: Vec<DockSpec<K>>,
}

/// One edge dock in a [`LayoutArea`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockSpec<K> {
    pub placement: crate::dock::model::DockPlacement,
    pub tree: LayoutTree<K>,
    /// Initial size along the dock's own axis. `None` uses the default.
    pub size: Option<f32>,
    /// Whether the dock starts open. Default `true`.
    pub open: bool,
    /// Whether the toggle affordance may close it. Default `true`.
    pub collapsible: bool,
}

impl<K: Copy> LayoutArea<K> {
    /// An area whose centre holds `center` and which has no edge docks.
    pub fn new(center: LayoutTree<K>) -> Self {
        Self {
            center,
            docks: Vec::new(),
        }
    }

    /// Add an edge dock of `size` pixels, open.
    #[must_use]
    pub fn dock(
        mut self,
        placement: crate::dock::model::DockPlacement,
        size: f32,
        tree: LayoutTree<K>,
    ) -> Self {
        self.docks.push(DockSpec {
            placement,
            tree,
            size: Some(size),
            open: true,
            collapsible: true,
        });
        self
    }

    /// Add an edge dock with a stated size and open flag.
    #[must_use]
    pub fn dock_with(
        mut self,
        placement: crate::dock::model::DockPlacement,
        size: Option<f32>,
        open: bool,
        tree: LayoutTree<K>,
    ) -> Self {
        self.docks.push(DockSpec {
            placement,
            tree,
            size,
            open,
            collapsible: true,
        });
        self
    }

    /// Mark a dock as not collapsible, so its toggle refuses to close it.
    #[must_use]
    pub fn not_collapsible(mut self, placement: crate::dock::model::DockPlacement) -> Self {
        for spec in &mut self.docks {
            if spec.placement == placement {
                spec.collapsible = false;
            }
        }
        self
    }
}

/// Validate a whole area before compilation.
pub(crate) fn validate_area<K>(area: &LayoutArea<K>) -> crate::dock::Result {
    validate_tree(&area.center)?;

    // Panel ids are unique across the whole area, not per region: the runtime index
    // is a single map, so two regions holding `"editor"` would leave one of them
    // unreachable by id — and it would be whichever the rebuild happened to visit
    // last, which is not something to leave to chance.
    let mut panel_ids = HashSet::new();
    collect_panel_ids(&area.center, &mut panel_ids)?;

    let mut seen = HashSet::new();
    for spec in &area.docks {
        if !spec.placement.is_dock() {
            return Err(Error::InvalidDockPlacement(spec.placement));
        }
        if !seen.insert(spec.placement) {
            return Err(Error::DuplicateDockPlacement(spec.placement));
        }
        validate_tree(&spec.tree)?;
        collect_panel_ids(&spec.tree, &mut panel_ids)?;
    }
    Ok(())
}

/// Gather a tree's panel ids into `seen`, rejecting one already there.
fn collect_panel_ids<K>(tree: &LayoutTree<K>, seen: &mut HashSet<String>) -> crate::dock::Result {
    match tree {
        LayoutTree::Tabs(node) => {
            for def in &node.panels {
                if !seen.insert(def.id.clone()) {
                    return Err(Error::DuplicatePanelId(def.id.clone()));
                }
            }
        }
        LayoutTree::Split(node) => {
            for child in &node.children {
                collect_panel_ids(child, seen)?;
            }
        }
    }
    Ok(())
}

/// Create a panel definition (for use inside [`tabs`]).
#[must_use]
pub fn panel_def<K: Copy>(
    id: impl Into<String>,
    title: impl Into<String>,
    content: K,
) -> PanelDef<K> {
    PanelDef::new(id, title, content)
}

/// Create a tabbed pane node.
pub fn tabs<K: Copy>(panels: impl IntoIterator<Item = PanelDef<K>>) -> LayoutTree<K> {
    LayoutTree::Tabs(TabsNode::new(panels))
}

/// Create a horizontal split.
pub fn horizontal<K>(children: impl IntoIterator<Item = LayoutTree<K>>) -> LayoutTree<K> {
    LayoutTree::Split(SplitNode::new(Axis::Horizontal, children))
}

/// Create a vertical split.
pub fn vertical<K>(children: impl IntoIterator<Item = LayoutTree<K>>) -> LayoutTree<K> {
    LayoutTree::Split(SplitNode::new(Axis::Vertical, children))
}

/// Single panel occupying the full dock area.
#[must_use]
pub fn single<K: Copy>(def: PanelDef<K>) -> LayoutTree<K> {
    LayoutTree::Tabs(TabsNode::new([def]))
}

/// Validate a layout tree before compilation.
pub(crate) fn validate_tree<K>(tree: &LayoutTree<K>) -> crate::dock::Result {
    let mut panel_ids = HashSet::new();
    let mut pane_names = HashSet::new();
    validate_node(tree, &mut panel_ids, &mut pane_names)
}

fn validate_node<K>(
    tree: &LayoutTree<K>,
    panel_ids: &mut HashSet<String>,
    pane_names: &mut HashSet<String>,
) -> crate::dock::Result {
    match tree {
        LayoutTree::Tabs(node) => {
            if node.panels.is_empty() {
                return Err(Error::EmptyLayout);
            }
            if let Some(name) = &node.name {
                if !pane_names.insert(name.clone()) {
                    return Err(Error::DuplicatePaneName(name.clone()));
                }
            }
            let pane_label = node.name.clone().unwrap_or_else(|| "<unnamed>".into());
            for def in &node.panels {
                if !panel_ids.insert(def.id.clone()) {
                    return Err(Error::DuplicatePanelId(def.id.clone()));
                }
            }
            if let Some(active) = &node.active {
                if !node.panels.iter().any(|p| p.id == *active) {
                    return Err(Error::UnknownActivePanel {
                        pane_name: pane_label,
                        panel: active.clone(),
                    });
                }
            }
            Ok(())
        }
        LayoutTree::Split(node) => {
            if node.children.is_empty() {
                return Err(Error::EmptyLayout);
            }
            if let Some(weights) = &node.weights {
                if weights.len() != node.children.len() {
                    return Err(Error::InvalidWeights {
                        expected: node.children.len(),
                        got: weights.len(),
                    });
                }
            }
            for child in &node.children {
                validate_node(child, panel_ids, pane_names)?;
            }
            Ok(())
        }
    }
}
