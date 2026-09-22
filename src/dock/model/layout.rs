// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::pane::{Pane, Panel};
use super::ProportionalGroup;

/// Stable node handle in the layout arena.
///
/// A plain global counter rather than a per-arena index, because a dock holds
/// several arenas at once — a centre and one per edge dock — and an id has to name
/// one node across all of them. With per-arena keys the centre's root and a dock's
/// root would both be id 1, so anything carrying an id (a zoom, an index entry, a
/// drag target) would be ambiguous between them. Ids are never reused, so a stale
/// id fails to resolve instead of silently naming a different node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeId(u64);

impl NodeId {
    /// The id of the next node this process creates.
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node#{}", self.0)
    }
}

/// Dock drop / split operations (floating window deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockOperation {
    Fill,
    Left,
    Right,
    Top,
    Bottom,
}

impl DockOperation {
    #[must_use]
    pub fn is_edge(self) -> bool {
        !matches!(self, Self::Fill)
    }
}

/// Split orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    #[must_use]
    pub fn perpendicular(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    #[must_use]
    pub fn for_operation(op: DockOperation) -> Option<Self> {
        match op {
            DockOperation::Left | DockOperation::Right => Some(Self::Horizontal),
            DockOperation::Top | DockOperation::Bottom => Some(Self::Vertical),
            DockOperation::Fill => None,
        }
    }
}

/// Root wrapper (single child).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RootState {
    pub child: Option<NodeId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeKind<K> {
    Panel(Panel<K>),
    Pane(Pane),
    Proportional(ProportionalGroup),
    Root(RootState),
}

#[derive(Debug)]
pub struct NodeEntry<K> {
    pub kind: NodeKind<K>,
    pub owner: Option<NodeId>,
}

impl<K: Clone> NodeEntry<K> {
    /// A copy of the entry, for staging a panel into a second tree while keeping
    /// the original for rollback.
    ///
    /// A dock's key is a plain value — a panel is named by an enum, not an owning
    /// handle — and every node kind owns clonable data, so the copy is always
    /// available where a cross-tree move needs it.
    #[must_use]
    pub fn clone_for_insert(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            owner: self.owner,
        }
    }
}

impl<K: Clone> Clone for NodeEntry<K> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            owner: self.owner,
        }
    }
}

/// Layout tree arena.
///
/// Nodes are kept in a map keyed by [`NodeId`]. Inserting mints a fresh id from the
/// global counter, so an id keeps its meaning for the life of the process and never
/// names a different node after a removal.
#[derive(Debug)]
pub struct Layout<K> {
    /// Every node the tree holds, keyed by its own id.
    pub nodes: HashMap<NodeId, NodeEntry<K>>,
    /// The node every walk starts from.
    pub root: NodeId,
}

/// Cloned by hand rather than derived, so the bound is `K: Clone` and not whatever a
/// `Serialize` derive happens to need. The derive would make a layout of a
/// non-serializable key unclonable, which is not a relationship worth having.
impl<K: Clone> Clone for Layout<K> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            root: self.root,
        }
    }
}

impl<K> Layout<K> {
    /// An empty layout: one root node and nothing under it.
    #[must_use]
    pub fn new() -> Self {
        let root = NodeId::next();
        let mut nodes = HashMap::new();
        nodes.insert(
            root,
            NodeEntry {
                kind: NodeKind::Root(RootState { child: None }),
                owner: None,
            },
        );
        Self { nodes, root }
    }

    /// Add a node and return its id.
    pub fn insert(&mut self, entry: NodeEntry<K>) -> NodeId {
        let id = NodeId::next();
        self.nodes.insert(id, entry);
        id
    }

    /// Put a node back under the id it already had.
    ///
    /// Used when a node moves between trees — the centre and a dock are separate
    /// arenas — where minting a fresh id would change the node's identity and strand
    /// every reference to it: the index, the zoom, an open drag session. The id is a
    /// process-wide handle, so carrying it across trees is what keeps "the same panel"
    /// meaning the same thing.
    pub fn insert_with_id(&mut self, id: NodeId, entry: NodeEntry<K>) -> Option<NodeEntry<K>> {
        self.nodes.insert(id, entry)
    }

    /// Take a node out of the arena. `None` when it was not there.
    pub fn remove(&mut self, id: NodeId) -> Option<NodeEntry<K>> {
        self.nodes.remove(&id)
    }

    #[must_use]
    pub fn contains_key(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&NodeEntry<K>> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeEntry<K>> {
        self.nodes.get_mut(&id)
    }

    #[must_use]
    pub fn kind(&self, id: NodeId) -> Option<&NodeKind<K>> {
        self.nodes.get(&id).map(|e| &e.kind)
    }

    pub fn set_owner(&mut self, id: NodeId, owner: Option<NodeId>) {
        if let Some(entry) = self.nodes.get_mut(&id) {
            entry.owner = owner;
        }
    }

    #[must_use]
    pub fn is_leaf(&self, id: NodeId) -> bool {
        matches!(self.kind(id), Some(NodeKind::Panel(_)))
    }

    #[must_use]
    pub fn root_child(&self) -> Option<NodeId> {
        match self.kind(self.root)? {
            NodeKind::Root(r) => r.child,
            _ => None,
        }
    }

    pub fn set_root_child(&mut self, child: Option<NodeId>) {
        if let Some(NodeKind::Root(ref mut r)) = self.nodes.get_mut(&self.root).map(|e| &mut e.kind)
        {
            r.child = child;
        }
        if let Some(child) = child {
            self.set_owner(child, Some(self.root));
        }
    }

    /// Every node id in the arena, in creation order.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        let mut ids: Vec<_> = self.nodes.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter()
    }
}

impl<K> Default for Layout<K> {
    fn default() -> Self {
        Self::new()
    }
}

/// Saved as a list of nodes rather than a map, so a file does not depend on a hash
/// map's iteration order and the ids stay readable as plain numbers.
#[cfg(feature = "dock-serde")]
impl<K> serde::Serialize for Layout<K>
where
    K: serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let sorted: Vec<_> = self.ids().map(|id| (id, &self.nodes[&id])).collect();
        let mut state = serializer.serialize_struct("Layout", 2)?;
        state.serialize_field("nodes", &sorted)?;
        state.serialize_field("root", &self.root)?;
        state.end()
    }
}

#[cfg(feature = "dock-serde")]
impl<'de, K> serde::Deserialize<'de> for Layout<K>
where
    K: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename = "Layout")]
        struct Repr<K> {
            nodes: Vec<(NodeId, NodeEntry<K>)>,
            root: NodeId,
        }
        let repr = Repr::deserialize(deserializer)?;
        Ok(Self {
            nodes: repr.nodes.into_iter().collect(),
            root: repr.root,
        })
    }
}

#[cfg(feature = "dock-serde")]
impl<K: serde::Serialize> serde::Serialize for NodeEntry<K> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let mut state = serializer.serialize_struct("NodeEntry", 2)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("owner", &self.owner)?;
        state.end()
    }
}

#[cfg(feature = "dock-serde")]
impl<'de, K: serde::Deserialize<'de>> serde::Deserialize<'de> for NodeEntry<K> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(rename = "NodeEntry")]
        struct Repr<K> {
            kind: NodeKind<K>,
            owner: Option<NodeId>,
        }
        let repr = Repr::deserialize(deserializer)?;
        Ok(Self {
            kind: repr.kind,
            owner: repr.owner,
        })
    }
}
