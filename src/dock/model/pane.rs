// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use super::NodeId;

/// Default for the `bool` fields that default to `true`, so a layout written
/// before the field existed still deserializes.
#[cfg(feature = "dock-serde")]
fn default_true() -> bool {
    true
}

/// Single tab content (leaf node).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Panel<K> {
    pub id: String,
    pub title: String,
    pub content: K,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_drop: bool,
    /// Whether the zoom affordance may maximize this panel.
    ///
    /// A refused zoom is reported back through the pane, which keeps its own
    /// `zoomed` flag in step with what the area actually did.
    #[cfg_attr(feature = "dock-serde", serde(default = "default_true"))]
    pub can_zoom: bool,
    /// Whether the tab bar offers this panel.
    ///
    /// A hidden panel stays in its pane and keeps its place, so unhiding it
    /// restores the layout the user had; it is simply left out of the tab strip
    /// and never becomes the displayed tab.
    #[cfg_attr(feature = "dock-serde", serde(default = "default_true"))]
    pub visible: bool,
    /// A short name for a group with no room for the full title.
    #[cfg_attr(feature = "dock-serde", serde(default))]
    pub tab_name: Option<String>,
    pub group: Option<String>,
}

impl<K: Copy> Panel<K> {
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

    /// The label a tab should show: the short name when there is one, the title
    /// otherwise.
    #[must_use]
    pub fn tab_label(&self) -> &str {
        self.tab_name.as_deref().unwrap_or(&self.title)
    }

    /// Whether the zoom affordance may maximize this panel. Default `true`.
    #[must_use]
    pub fn can_zoom(mut self, value: bool) -> Self {
        self.can_zoom = value;
        self
    }

    /// Whether the tab bar offers this panel. Default `true`.
    #[must_use]
    pub fn visible(mut self, value: bool) -> Self {
        self.visible = value;
        self
    }

    #[must_use]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// Tabbed pane host.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pane {
    pub name: Option<String>,
    pub tabs: Vec<NodeId>,
    pub active: Option<NodeId>,
    pub group: Option<String>,
    pub persistent: bool,
}

impl Default for Pane {
    fn default() -> Self {
        Self::new()
    }
}

impl Pane {
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: None,
            tabs: Vec::new(),
            active: None,
            group: None,
            persistent: false,
        }
    }

    #[must_use]
    pub fn active_index(&self) -> Option<usize> {
        let active = self.active?;
        self.tabs.iter().position(|&id| id == active)
    }
}
