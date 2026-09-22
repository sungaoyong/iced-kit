// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Saving a workspace and restoring it.
//!
//! A docking layout is the one piece of an application's state a user notices when
//! it is lost: reopening a project and finding every panel back where it was is the
//! point of having docks at all. [`Layout`] already derives `Serialize` under the
//! `dock-serde` feature, so a single tree is covered; this module adds what a tree
//! cannot describe on its own — the edge docks, their sizes, whether they were
//! open, and which panel was maximized.
//!
//! # What is saved, and what deliberately is not
//!
//! Saved: every panel and its place, each dock's size and open flag, whether a dock
//! may be collapsed, and the zoomed panel's key.
//!
//! Not saved: `hidden` (runtime visibility), the current drag, and focus. Those are
//! all things a user expects to be *current* on reopen rather than restored —
//! restoring a hidden panel from a session three launches ago would read as a bug.
//!
//! # Node ids
//!
//! [`NodeId`] is a process-wide counter, so the ids in a restored tree are not the
//! ids that were saved. Nothing an application should depend on breaks: panel keys
//! and pane names are the stable handles, which is what [`DockAreaState::zoomed`]
//! stores — a panel key, not a node id.

use crate::dock::model::{Dock, DockPlacement, DockRegion, DockRegions, Layout, NodeId, NodeKind};
use crate::dock::{Error, Result};

/// A whole dock area as plain data: the centre, every dock, and the zoom.
///
/// Versioned so a later build can recognise a file it wrote before. The dock only
/// compares the number; what a version *means* is the application's to decide.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockAreaState<K> {
    /// The application's layout version, when it has one.
    #[cfg_attr(feature = "dock-serde", serde(default))]
    pub version: Option<usize>,
    /// The centre tree.
    pub center: Layout<K>,
    /// Each edge dock that existed.
    #[cfg_attr(feature = "dock-serde", serde(default))]
    pub docks: Vec<DockSlot<K>>,
    /// The key of the panel that was maximized, if any.
    #[cfg_attr(feature = "dock-serde", serde(default))]
    pub zoomed: Option<String>,
}

/// One edge dock as saved.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockSlot<K> {
    pub placement: DockPlacement,
    pub tree: Layout<K>,
    /// Size along the dock's own axis, in logical pixels.
    pub size: f32,
    pub open: bool,
    /// Whether the toggle affordance may close it. Defaults to `true`, so a file
    /// written before the field existed restores as it looked.
    #[cfg_attr(feature = "dock-serde", serde(default = "default_true"))]
    pub collapsible: bool,
}

#[cfg(feature = "dock-serde")]
fn default_true() -> bool {
    true
}

impl<K: Clone> DockAreaState<K> {
    /// Save a whole area.
    ///
    /// The zoom is recorded as a *panel key* rather than a node id: an id is a
    /// process-wide counter and means nothing after a restart, while a key is the
    /// handle the application chose and persists.
    #[must_use]
    pub fn capture(center: &Layout<K>, regions: &DockRegions<K>, version: Option<usize>) -> Self
    where
        K: Clone,
    {
        let mut docks = Vec::new();
        for (placement, region) in regions.iter() {
            let Some(dock) = regions.dock(placement) else {
                continue;
            };
            docks.push(DockSlot {
                placement,
                tree: region.tree.clone(),
                size: dock.size(),
                open: dock.is_open(),
                collapsible: dock.is_collapsible(),
            });
        }

        Self {
            version,
            center: center.clone(),
            docks,
            zoomed: zoomed_panel_key(center, regions.zoomed()),
        }
    }
}

/// The key of the panel a zoomed group displays.
fn zoomed_panel_key<K>(center: &Layout<K>, zoomed: Option<NodeId>) -> Option<String> {
    let node = zoomed?;
    let panel = match center.kind(node) {
        Some(NodeKind::Pane(pane)) => pane.active.or_else(|| pane.tabs.first().copied()),
        // A zoomed node that is not a pane is not something a key can describe;
        // the zoom is simply not saved, which restores to the layout.
        _ => None,
    }?;
    match center.kind(panel) {
        Some(NodeKind::Panel(panel)) => Some(panel.id.clone()),
        _ => None,
    }
}

impl<K> DockRegions<K> {
    /// Replace every dock from saved state.
    ///
    /// The docks that were there are dropped, including any dock the saved state
    /// does not mention: a workspace restored from a file describes a *whole*
    /// layout, so a dock absent from it was not part of the workspace.
    pub fn restore(&mut self, saved: &[DockSlot<K>]) -> Result<()>
    where
        K: Clone,
    {
        for slot in saved {
            if !slot.placement.is_dock() {
                return Err(Error::InvalidDockPlacement(slot.placement));
            }
        }

        self.clear();
        for slot in saved {
            let mut dock = Dock::new(slot.size);
            dock.set_open(slot.open);
            dock.set_collapsible(slot.collapsible);
            self.insert_dock(
                slot.placement,
                DockRegion {
                    tree: slot.tree.clone(),
                    dock,
                },
            );
        }
        Ok(())
    }

    /// Take every dock away.
    pub fn clear(&mut self) {
        for placement in DockPlacement::DOCKS {
            self.remove_dock(placement);
        }
    }

    /// Restore the zoom that was saved.
    ///
    /// `zoomed` must already be resolved from the saved panel *key* to the pane that
    /// holds it — the caller owns the index the key is looked up in, and doing the
    /// lookup here would mean borrowing the layout it sits beside.
    ///
    /// A key nothing holds any more resolves to `None`, which leaves the area
    /// unzoomed rather than failing the load: a workspace whose maximized panel was
    /// removed by a later build should still open.
    pub fn restore_zoom(&mut self, zoomed: Option<NodeId>) {
        self.set_zoomed(zoomed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock::model::Axis;

    fn area<K: Copy>(k: K) -> crate::dock::LayoutArea<K> {
        crate::dock::LayoutArea::new(crate::dock::tabs([crate::dock::panel("a", "A", k)])).dock(
            DockPlacement::Left,
            240.0,
            crate::dock::tabs([crate::dock::panel("b", "B", k)]),
        )
    }

    #[test]
    fn capturing_and_restoring_keeps_every_dock() {
        let (built, regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        let state = DockAreaState::capture(&built.layout, &regions, Some(3));

        assert_eq!(state.version, Some(3));
        assert_eq!(state.docks.len(), 1);
        assert_eq!(state.docks[0].placement, DockPlacement::Left);
        assert_eq!(state.docks[0].size, 240.0);

        let mut restored: DockRegions<u32> = DockRegions::new();
        restored.restore(&state.docks).expect("restores");
        assert!(restored.has_dock(DockPlacement::Left));
        assert_eq!(restored.dock(DockPlacement::Left).unwrap().size(), 240.0);
    }

    #[test]
    fn a_docks_open_flag_survives_a_round_trip() {
        let (built, mut regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        regions.dock_mut(DockPlacement::Left).unwrap().set_open(false);
        regions
            .dock_mut(DockPlacement::Left)
            .unwrap()
            .set_collapsible(false);

        let state = DockAreaState::capture(&built.layout, &regions, None);
        let mut restored: DockRegions<u32> = DockRegions::new();
        restored.restore(&state.docks).expect("restores");

        assert!(!restored.is_dock_open(DockPlacement::Left));
        assert!(!restored.is_dock_collapsible(DockPlacement::Left));
    }

    #[test]
    fn restoring_drops_a_dock_the_saved_area_did_not_have() {
        // A saved workspace describes the whole layout, so a dock absent from it was
        // not part of it — leaving one behind would show a panel the file never had.
        let (built, regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        // Saved while only the left dock existed.
        let state = DockAreaState::capture(&built.layout, &regions, None);

        // The live area gains a dock the file knows nothing about.
        let mut restored: DockRegions<u32> = DockRegions::new();
        restored.restore(&state.docks).expect("restores");
        restored.insert_dock(DockPlacement::Right, DockRegion::new(Dock::new(300.0)));
        assert!(restored.has_dock(DockPlacement::Right));

        restored.restore(&state.docks).expect("restores again");

        assert!(restored.has_dock(DockPlacement::Left));
        assert!(
            !restored.has_dock(DockPlacement::Right),
            "the saved state did not have a right dock"
        );
    }

    #[test]
    fn a_saved_centre_keeps_its_panels_and_its_split_axis() {
        let tree = crate::dock::vertical([
            crate::dock::tabs([crate::dock::panel("top", "Top", 0u32)]),
            crate::dock::tabs([crate::dock::panel("bottom", "Bottom", 1u32)]),
        ]);
        let (built, regions) = crate::dock::builder::compile::build_area(
            &crate::dock::LayoutArea::new(tree),
        )
        .expect("valid");
        let state = DockAreaState::capture(&built.layout, &regions, None);

        let root = state.center.root_child().expect("a root child");
        assert!(
            matches!(state.center.kind(root), Some(NodeKind::Proportional(pg)) if pg.axis == Axis::Vertical),
            "the split axis is part of the layout"
        );
    }

    #[test]
    fn the_zoom_is_saved_by_panel_key_not_by_node_id() {
        let (built, mut regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        let pane = crate::dock::builder::compile::first_pane(&built.layout).expect("a pane");
        regions.set_zoomed(Some(pane));

        let state = DockAreaState::capture(&built.layout, &regions, None);
        assert_eq!(state.zoomed.as_deref(), Some("a"));
    }

    #[test]
    fn a_restored_zoom_whose_panel_is_gone_leaves_the_area_unzoomed() {
        let (_, mut regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        // A key nothing holds any more resolves to no pane at all.
        regions.restore_zoom(None);
        assert_eq!(regions.zoomed(), None);
    }

    #[test]
    fn a_restored_zoom_is_recorded_against_the_pane() {
        let (built, mut regions) =
            crate::dock::builder::compile::build_area(&area(0u32)).expect("valid");
        let pane = crate::dock::builder::compile::first_pane(&built.layout).expect("a pane");
        regions.restore_zoom(Some(pane));
        assert_eq!(regions.zoomed(), Some(pane));
    }

    #[test]
    fn a_centre_placement_is_not_a_dock_and_is_rejected() {
        let mut regions: DockRegions<u32> = DockRegions::new();
        let error = regions
            .restore(&[DockSlot {
                placement: DockPlacement::Center,
                tree: Layout::new(),
                size: 200.0,
                open: true,
                collapsible: true,
            }])
            .expect_err("the centre is not a dock");
        assert!(matches!(error, Error::InvalidDockPlacement(_)));
    }
}
