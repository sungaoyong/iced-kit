// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use crate::dock::builder::index::DockIndex;
use crate::dock::builder::spec::{
    validate_area, validate_tree, LayoutArea, LayoutTree, PanelDef, SplitNode, TabsNode,
};
use crate::dock::factory::Factory;
use crate::dock::model::{Dock, DockRegion, DockRegions, Layout, NodeId, NodeKind, Pane};
use crate::dock::widget::DockWidgetState;
use crate::dock::{Error, Result};

/// Result of compiling a [`LayoutTree`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BuiltLayout<K> {
    pub layout: Layout<K>,
    pub index: DockIndex,
}

/// Compile a declarative [`LayoutTree`] into a runtime [`Layout`] and index.
pub fn build_tree<K: Copy>(tree: &LayoutTree<K>) -> Result<BuiltLayout<K>> {
    validate_tree(tree)?;
    let factory = Factory;
    let mut layout = Layout::new();
    let mut index = DockIndex::default();
    let root = compile_node(tree, &factory, &mut layout, &mut index)?;
    layout.set_root_child(Some(root));
    Ok(BuiltLayout { layout, index })
}

/// Compile a whole [`LayoutArea`] — a centre tree plus its edge docks.
///
/// The centre and the docks are returned separately, because that is how the
/// widget state holds them: the centre is the main layout, and the docks hang
/// beside it. Each dock ends up with its own independent [`Layout`], so the trees
/// stay separable values. Every panel across every region shares one index,
/// because a panel id is unique across the whole area, not per region.
pub fn build_area<K: Copy>(area: &LayoutArea<K>) -> Result<(BuiltLayout<K>, DockRegions<K>)> {
    validate_area(area)?;
    let factory = Factory;
    let mut index = DockIndex::default();
    let mut center = Layout::new();

    let center_root = compile_node(&area.center, &factory, &mut center, &mut index)?;
    center.set_root_child(Some(center_root));

    let mut regions = DockRegions::new();
    for spec in &area.docks {
        let mut tree = Layout::new();
        let root = compile_node(&spec.tree, &factory, &mut tree, &mut index)?;
        tree.set_root_child(Some(root));

        let mut dock = Dock::new(spec.size.unwrap_or_else(Dock::default_size));
        dock.set_open(spec.open);
        dock.set_collapsible(spec.collapsible);
        regions.insert_dock(spec.placement, DockRegion { tree, dock });
    }

    Ok((
        BuiltLayout {
            layout: center,
            index,
        },
        regions,
    ))
}

fn compile_node<K: Copy>(
    tree: &LayoutTree<K>,
    factory: &Factory,
    layout: &mut Layout<K>,
    index: &mut DockIndex,
) -> Result<NodeId> {
    match tree {
        LayoutTree::Tabs(node) => compile_tabs(node, factory, layout, index),
        LayoutTree::Split(node) => compile_split(node, factory, layout, index),
    }
}

fn compile_tabs<K: Copy>(
    node: &TabsNode<K>,
    factory: &Factory,
    layout: &mut Layout<K>,
    index: &mut DockIndex,
) -> Result<NodeId> {
    let pane_id = factory.create_pane(layout);
    if let Some(NodeKind::Pane(ref mut pane)) = layout.get_mut(pane_id).map(|e| &mut e.kind) {
        pane.name.clone_from(&node.name);
        pane.group.clone_from(&node.group);
        pane.persistent = node.persistent;
    }
    if let Some(name) = &node.name {
        index.panes.insert(name.clone(), pane_id);
    }

    let mut panel_nodes = Vec::with_capacity(node.panels.len());
    for def in &node.panels {
        let panel_id = insert_panel(factory, layout, index, def);
        // Panels with no declared group inherit the pane's group.
        if def.group.is_none() {
            if let (Some(pane_group), Some(NodeKind::Panel(ref mut panel))) =
                (&node.group, layout.get_mut(panel_id).map(|e| &mut e.kind))
            {
                panel.group = Some(pane_group.clone());
            }
        }
        factory.add_panel_to_pane(layout, pane_id, panel_id)?;
        panel_nodes.push((def.id.clone(), panel_id));
    }

    if let Some(active) = &node.active {
        if let Some((_, panel_id)) = panel_nodes.iter().find(|(id, _)| id == active) {
            factory.set_active_panel(layout, pane_id, *panel_id);
        }
    }

    Ok(pane_id)
}

fn compile_split<K: Copy>(
    node: &SplitNode<K>,
    factory: &Factory,
    layout: &mut Layout<K>,
    index: &mut DockIndex,
) -> Result<NodeId> {
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        children.push(compile_node(child, factory, layout, index)?);
    }
    let group_id = factory.create_proportional(layout, node.axis, children);
    if let Some(weights) = &node.weights {
        factory.set_proportions(layout, group_id, weights.clone())?;
    }
    Ok(group_id)
}

fn insert_panel<K: Copy>(
    factory: &Factory,
    layout: &mut Layout<K>,
    index: &mut DockIndex,
    def: &PanelDef<K>,
) -> NodeId {
    let panel_id = factory.insert_panel(layout, def.id.clone(), def.title.clone(), def.content);
    if let Some(NodeKind::Panel(ref mut panel)) = layout.get_mut(panel_id).map(|e| &mut e.kind) {
        panel.can_close = def.can_close;
        panel.can_drag = def.can_drag;
        panel.can_drop = def.can_drop;
        panel.can_zoom = def.can_zoom;
        panel.visible = def.visible;
        panel.tab_name.clone_from(&def.tab_name);
        panel.group.clone_from(&def.group);
    }
    index.panels.insert(def.id.clone(), panel_id);
    panel_id
}

/// Insert a panel using widget state (avoids overlapping field borrows).
pub(crate) fn insert_panel_into_state<K: Copy>(
    factory: &Factory,
    state: &mut DockWidgetState<K>,
    def: &PanelDef<K>,
) -> Result<NodeId> {
    if state.index.panels.contains_key(&def.id) {
        return Err(Error::DuplicatePanelId(def.id.clone()));
    }
    Ok(insert_panel(
        factory,
        &mut state.layout,
        &mut state.index,
        def,
    ))
}

/// Resolve the first pane in preorder tree walk.
#[must_use]
pub fn first_pane<K>(layout: &Layout<K>) -> Option<NodeId> {
    first_pane_where(layout, |_, _| true)
}

/// Resolve the first pane in preorder tree walk that satisfies `pred`.
pub(crate) fn first_pane_where<K>(
    layout: &Layout<K>,
    pred: impl Fn(NodeId, &Pane) -> bool,
) -> Option<NodeId> {
    let root = layout.root_child()?;
    first_pane_walk(layout, root, &pred)
}

fn first_pane_walk<K>(
    layout: &Layout<K>,
    node: NodeId,
    pred: &impl Fn(NodeId, &Pane) -> bool,
) -> Option<NodeId> {
    match layout.kind(node)? {
        NodeKind::Pane(pane) => pred(node, pane).then_some(node),
        NodeKind::Proportional(pg) => {
            for &child in &pg.children {
                if let Some(found) = first_pane_walk(layout, child, pred) {
                    return Some(found);
                }
            }
            None
        }
        NodeKind::Panel(_) | NodeKind::Root(_) => None,
    }
}

/// Find the pane that owns a panel node.
#[must_use]
pub fn owning_pane<K>(layout: &Layout<K>, panel: NodeId) -> Option<NodeId> {
    let e = layout.get(panel)?;
    e.owner
}

/// Pane that owns a panel identified by string id.
#[must_use]
pub fn pane_for_panel<K>(layout: &Layout<K>, index: &DockIndex, panel_id: &str) -> Option<NodeId> {
    let panel = index.panel_node(panel_id)?;
    owning_pane(layout, panel)
}

/// Active panel id string in a specific pane.
#[must_use]
pub fn active_panel_in_pane<K>(
    layout: &Layout<K>,
    index: &DockIndex,
    pane: NodeId,
) -> Option<String> {
    let NodeKind::Pane(pane_state) = layout.kind(pane)? else {
        return None;
    };
    let active = pane_state
        .active
        .or_else(|| pane_state.tabs.first().copied())?;
    index
        .panels
        .iter()
        .find_map(|(id, node_id)| (*node_id == active).then(|| id.clone()))
}

/// Whether a panel node is offered by its tab bar.
///
/// A hidden panel keeps its node and its slot; it is simply left out of the
/// strip. Anything that is not a panel leaf counts as visible, so a caller that
/// walks a tree generically needs no special case.
#[must_use]
pub fn panel_is_visible<K>(layout: &Layout<K>, panel: NodeId) -> bool {
    match layout.kind(panel) {
        Some(NodeKind::Panel(p)) => p.visible,
        _ => true,
    }
}

/// The panel a pane should display, given that hidden panels are passed over.
///
/// The stored `active` wins when it is visible. Otherwise the first visible tab
/// takes over, and `None` means every tab is hidden — the pane has nothing to
/// show, which is not the same as being empty.
#[must_use]
pub fn displayed_panel<K>(layout: &Layout<K>, pane: &Pane) -> Option<NodeId> {
    if let Some(active) = pane.active {
        if pane.tabs.contains(&active) && panel_is_visible(layout, active) {
            return Some(active);
        }
    }
    pane.tabs
        .iter()
        .copied()
        .find(|&id| panel_is_visible(layout, id))
}

/// The visible tabs of a pane, in order.
pub fn visible_tabs<'a, K>(
    layout: &'a Layout<K>,
    pane: &'a Pane,
) -> impl Iterator<Item = NodeId> + 'a {
    pane.tabs
        .iter()
        .copied()
        .filter(move |&id| panel_is_visible(layout, id))
}

/// Every panel node in a tree, in preorder.
pub fn panels_in_tree<K>(layout: &Layout<K>) -> Vec<NodeId> {
    let mut found = Vec::new();
    if let Some(root) = layout.root_child() {
        collect_panels(layout, root, &mut found);
    }
    found
}

fn collect_panels<K>(layout: &Layout<K>, node: NodeId, found: &mut Vec<NodeId>) {
    match layout.kind(node) {
        Some(NodeKind::Panel(_)) => found.push(node),
        Some(NodeKind::Pane(pane)) => {
            for &tab in &pane.tabs {
                collect_panels(layout, tab, found);
            }
        }
        Some(NodeKind::Proportional(pg)) => {
            for &child in &pg.children {
                collect_panels(layout, child, found);
            }
        }
        Some(NodeKind::Root(_)) | None => {}
    }
}

/// Whether a pane would draw nothing — every tab hidden, or no tabs at all.
#[must_use]
pub fn pane_is_empty<K>(layout: &Layout<K>, pane: &Pane) -> bool {
    !pane.tabs.iter().any(|&id| panel_is_visible(layout, id))
}
