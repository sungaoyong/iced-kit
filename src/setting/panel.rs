//! The settings panel: a sidebar, a search box, and the selected page.

use crate::icons::IconName;
use crate::setting::SettingPage;
use crate::theme::{Size, Theme};
use crate::widgets::GroupBoxVariant;
use crate::widgets::{Button, Icon};
use iced::widget::{column, container, row, rule, scrollable, text, Space};
use iced::{Element, Length, Padding, Task};

/// The width at which the sidebar stops sitting beside the content and stacks
/// above it.
///
/// Below this the two columns would each be too narrow to read, so the panel
/// gives the content the full width and puts navigation on top. The measurement
/// is of the panel rather than the window, so a settings page inside a narrow
/// dock pane stacks too.
pub const STACKED_LAYOUT_MAX_WIDTH: f32 = 560.0;

/// The tallest the sidebar is allowed to grow when the panel is stacked.
///
/// Stacked, the sidebar sits above the content and sizes to its own contents, so
/// this is a cap rather than a height: it keeps a long page list from pushing the
/// content out of view. The list scrolls within it.
pub const STACKED_SIDEBAR_MAX_HEIGHT: f32 = 240.0;

/// What the user did to the panel.
///
/// These describe changes to the panel's own view — which page is open, what is
/// being searched for. They carry no setting values: a field reports its own
/// changes through the message the caller gave it, so the two never mix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsEvent {
    /// A page was chosen in the sidebar.
    SelectPage(usize),
    /// A group was chosen in the sidebar, which also opens its page.
    SelectGroup {
        /// The page the group belongs to.
        page: usize,
        /// The group's index within that page, in the caller's own numbering.
        group: usize,
    },
    /// The search query changed.
    Search(String),
}

/// The part of a settings panel the caller owns.
///
/// iced widgets do not own state across frames, so the panel's view state —
/// which page is open, the search query, which group was last chosen — lives
/// here. This is the same shape as [`TableState`](crate::widgets::TableState)
/// and [`VirtualListState`](crate::widgets::VirtualListState).
///
/// [`apply`](SettingsState::apply) does the bookkeeping, so an application only
/// has to forward the event it was handed:
///
/// ```
/// use iced_kit::setting::{Settings, SettingsEvent, SettingsState};
/// use iced::Task;
///
/// #[derive(Clone, Debug)]
/// enum Message {
///     Settings(SettingsEvent),
/// }
///
/// struct App {
///     settings: SettingsState,
/// }
///
/// fn update(app: &mut App, message: Message) -> Task<Message> {
///     match message {
///         Message::Settings(event) => {
///             app.settings.apply(event);
///
///             match app.settings.take_pending_scroll() {
///                 Some((page, group)) => Settings::<Message>::scroll_to_group(page, group),
///                 None => Task::none(),
///             }
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    selected_page: usize,
    selected_group: Option<usize>,
    query: String,
    /// A group the caller asked to scroll to, cleared once acted on.
    pending_scroll: Option<(usize, usize)>,
}

impl SettingsState {
    /// Creates a state showing the first page with no search.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The page the user is looking at.
    #[must_use]
    pub fn selected_page(&self) -> usize {
        self.selected_page
    }

    /// The group the user last chose in the sidebar, if any.
    #[must_use]
    pub fn selected_group(&self) -> Option<usize> {
        self.selected_group
    }

    /// The current search query.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Opens a page directly.
    ///
    /// This is for an application that navigates to a settings page from
    /// elsewhere — a menu item, say — rather than through the sidebar.
    pub fn select_page(&mut self, page: usize) {
        self.selected_page = page;
        self.selected_group = None;
    }

    /// Sets the search query directly.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    /// Applies an event from the panel.
    ///
    /// Choosing a group also opens its page, so a caller that only forwards
    /// events never has to reconcile the two indices itself.
    pub fn apply(&mut self, event: SettingsEvent) {
        match event {
            SettingsEvent::SelectPage(page) => {
                self.selected_page = page;
                self.selected_group = None;
            }
            SettingsEvent::SelectGroup { page, group } => {
                self.selected_page = page;
                self.selected_group = Some(group);
                self.pending_scroll = Some((page, group));
            }
            SettingsEvent::Search(query) => {
                self.query = query;
            }
        }
    }

    /// Takes the group the panel should scroll to, if one was requested.
    ///
    /// Scrolling is a second step because a group's position is only known once
    /// iced has laid it out. The caller takes the target here and returns
    /// [`Settings::scroll_to_group`] from `update`; taking it clears it, so the
    /// scroll is not repeated on the next frame.
    pub fn take_pending_scroll(&mut self) -> Option<(usize, usize)> {
        self.pending_scroll.take()
    }
}

/// A settings panel.
///
/// Build it from an application's [`SettingsState`] and its pages.
///
/// ```
/// # use iced_kit::setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsEvent, SettingsState};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Panel(SettingsEvent), Dark(bool) }
/// # struct App { settings: SettingsState, dark: bool }
/// fn view(app: &App) -> Element<'_, Message, Theme> {
///     Settings::new(&app.settings)
///         .on_event(Message::Panel)
///         .page(
///             SettingPage::new("Appearance").group(
///                 SettingGroup::new()
///                     .title("Theme")
///                     .item(SettingItem::new("Dark mode").field(SettingField::switch(
///                         app.dark,
///                         Message::Dark,
///                     ))),
///             ),
///         )
///         .into()
/// }
/// ```
#[must_use = "a Settings panel does nothing unless it is turned into an Element"]
pub struct Settings<'a, Message> {
    state: &'a SettingsState,
    pages: Vec<SettingPage<'a, Message>>,
    size: Size,
    sidebar_width: f32,
    variant: GroupBoxVariant,
    disabled: bool,
    stacked: bool,
    on_event: Option<std::rc::Rc<dyn Fn(SettingsEvent) -> Message + 'a>>,
    on_reset: Option<std::rc::Rc<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Settings<'a, Message> {
    /// Creates a panel showing the pages of `state`.
    pub fn new(state: &'a SettingsState) -> Self {
        Self {
            state,
            pages: Vec::new(),
            size: Size::Md,
            sidebar_width: 220.0,
            variant: GroupBoxVariant::default(),
            disabled: false,
            stacked: false,
            on_event: None,
            on_reset: None,
        }
    }

    /// Adds a page.
    pub fn page(mut self, page: SettingPage<'a, Message>) -> Self {
        self.pages.push(page);
        self
    }

    /// Adds several pages.
    pub fn pages<I>(mut self, pages: I) -> Self
    where
        I: IntoIterator<Item = SettingPage<'a, Message>>,
    {
        self.pages.extend(pages);
        self
    }

    /// Sets the control size every field on the page uses.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the sidebar's width.
    ///
    /// iced's split panes resize proportionally rather than within a pixel
    /// range, so this width is fixed rather than user-draggable.
    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width.max(120.0);
        self
    }

    /// Overrides the surface every group draws on.
    pub fn group_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Draws every control in the panel inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Reports navigation and search changes.
    ///
    /// Without this the panel still draws, but its sidebar and search box do
    /// nothing.
    pub fn on_event(mut self, on_event: impl Fn(SettingsEvent) -> Message + 'a) -> Self {
        self.on_event = Some(std::rc::Rc::new(on_event));
        self
    }

    /// Reports a page's "reset all" being pressed.
    ///
    /// The panel cannot reset values it does not own, so it reports the page
    /// index and the application restores its own defaults. The control only
    /// appears while the page actually has a changed, resettable setting.
    pub fn on_reset(mut self, on_reset: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_reset = Some(std::rc::Rc::new(on_reset));
        self
    }

    /// The pages that survive the current query.
    ///
    /// Filtering never renumbers the caller's data: an entry pairs the page's
    /// index in the original list with the page itself, so an event always
    /// refers to the index the application used.
    fn visible_pages(&self) -> Vec<(usize, &SettingPage<'a, Message>)> {
        let query = &self.state.query;

        self.pages
            .iter()
            .enumerate()
            .filter(|(_, page)| crate::setting::search::page_matches(page, query))
            .collect()
    }

    /// The position, within the visible list, of the page to draw.
    ///
    /// A search can hide the selected page, so this falls back to the first
    /// visible one rather than drawing nothing.
    fn active_page(&self, visible: &[(usize, &SettingPage<'a, Message>)]) -> Option<usize> {
        if visible.is_empty() {
            return None;
        }

        visible
            .iter()
            .position(|(ix, _)| *ix == self.state.selected_page)
            .or(Some(0))
    }

    /// Draws the sidebar: the search box and the page list.
    fn render_sidebar(
        &self,
        visible: &[(usize, &SettingPage<'a, Message>)],
    ) -> Element<'a, Message, Theme> {
        let query = self.state.query.clone();
        let selected_page = self.state.selected_page;
        let selected_group = self.state.selected_group;

        let mut search = crate::widgets::text_input::<Message>("Search settings", &query)
            .size(self.size)
            .width(Length::Fill)
            .prefix(Icon::new(IconName::Search).into_element(Size::Sm));

        if let Some(on_event) = self.on_event.clone() {
            search = search.on_input(move |value| on_event(SettingsEvent::Search(value)));
        }

        let mut list = column![].spacing(2).width(Length::Fill);

        for (page_ix, page) in visible {
            let is_active = *page_ix == selected_page;

            // With no event handler the panel is a static preview, so the same
            // row is drawn as plain content rather than a button that would do
            // nothing when pressed.
            let entry: Element<'a, Message, Theme> = if let Some(on_event) = self.on_event.clone() {
                let mut button = Button::new(page.title_text().to_owned())
                    .size(self.size)
                    .width(Length::Fill)
                    .selected(is_active)
                    .ghost();

                if let Some(icon) = page.icon_name() {
                    button = button.icon(Icon::new(icon));
                }

                button
                    .push(Space::new().width(Length::Fill))
                    .on_press(on_event(SettingsEvent::SelectPage(*page_ix)))
                    .into()
            } else {
                let mut label = row![].spacing(8).align_y(iced::Alignment::Center);

                if let Some(icon) = page.render_icon(self.size) {
                    label = label.push(icon);
                }

                label = label.push(
                    text(page.title_text().to_owned()).size(self.size.text().size),
                );

                container(label).padding(Padding::from(8)).into()
            };

            list = list.push(entry);

            // A page with one group needs no sub-entries: the group is the page,
            // and listing it would add a level with nothing to choose between.
            if page.groups.len() > 1 {
                let mut groups = column![].spacing(1).width(Length::Fill);

                for (group_ix, group) in page.groups.iter().enumerate() {
                    if !crate::setting::search::group_matches(group, &query) {
                        continue;
                    }

                    // An untitled group has no name to list. It is still
                    // reachable by scrolling, just not by a label.
                    let Some(title) = group.title_text() else {
                        continue;
                    };

                    let is_group_active = is_active && selected_group == Some(group_ix);

                    let entry: Element<'a, Message, Theme> = match self.on_event.clone() {
                        Some(on_event) => Button::new(title.to_owned())
                            .size(Size::Sm)
                            .width(Length::Fill)
                            .selected(is_group_active)
                            .text()
                            .push(Space::new().width(Length::Fill))
                            .on_press(on_event(SettingsEvent::SelectGroup {
                                page: *page_ix,
                                group: group_ix,
                            }))
                            .into(),
                        None => container(text(title.to_owned()).size(Size::Sm.text().size))
                            .padding(Padding {
                                top: 4.0,
                                right: 8.0,
                                bottom: 4.0,
                                left: 24.0,
                            })
                            .into(),
                    };

                    groups = groups.push(entry);
                }

                list = list.push(container(groups).padding(Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 4.0,
                    left: 12.0,
                }));
            }
        }

        // Beside the content the sidebar fills its column; stacked it sizes to
        // its own contents, or `max_height` would reserve the cap even for a
        // single page and leave a gap above the page it opens.
        let list_height = if self.stacked {
            Length::Shrink
        } else {
            Length::Fill
        };

        container(
            column![
                search,
                Space::new().height(Length::Fixed(12.0)),
                scrollable(list).height(list_height),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(list_height)
        .padding(Padding::from(12))
        .into()
    }

    /// Draws the selected page: its header, then its groups.
    fn render_content(
        &self,
        visible: &[(usize, &SettingPage<'a, Message>)],
    ) -> Element<'a, Message, Theme> {
        let Some(active) = self.active_page(visible) else {
            return Self::render_empty();
        };

        let query = self.state.query.clone();
        let (page_ix, page) = visible[active];

        let reset: Option<Element<'a, Message, Theme>> = self.on_reset.clone().map(|on_reset| {
            Button::new("Reset")
                .size(Size::Sm)
                .ghost()
                .icon(Icon::new(IconName::Undo2))
                .on_press(on_reset(page_ix))
                .into()
        });

        let header = page.render_header(&query, self.size, reset);

        let groups = page.render_groups(
            &query,
            self.size,
            self.variant,
            self.disabled,
            |group_ix, element| {
                // Each group is wrapped in an identified container so
                // `scroll_to_group` can find it.
                container(element).id(group_id(page_ix, group_ix)).into()
            },
        );

        scrollable(column![header, groups].spacing(8).width(Length::Fill))
            .id(content_id())
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    /// Draws the "nothing matches" state.
    fn render_empty() -> Element<'a, Message, Theme> {
        container(
            text("No settings match your search.")
                .size(Size::Md.text().size)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::from(24))
        .into()
    }

    /// Draws the panel stacked: navigation above the content instead of beside
    /// it.
    ///
    /// The reference decides this from the panel's own width, which iced cannot
    /// do for a composed panel — a layout-aware closure would have to rebuild
    /// the pages on each pass, and iced's `Element` borrows the pages rather than
    /// owning them, so it cannot be built inside such a closure.
    ///
    /// The application already knows its window's width, so it decides instead:
    /// track resize events and pass the result here. The threshold is exposed as
    /// [`STACKED_LAYOUT_MAX_WIDTH`].
    ///
    /// ```
    /// # use iced_kit::setting::{Settings, SettingsState, STACKED_LAYOUT_MAX_WIDTH};
    /// # use iced::window;
    /// # #[derive(Clone, Debug)] enum Message { Resized(iced::Size) }
    /// # struct App { settings: SettingsState, window_width: f32 }
    /// # fn update(app: &mut App, event: window::Event) {
    /// if let window::Event::Resized(size) = event {
    ///     app.window_width = size.width;
    /// }
    /// # }
    /// # fn view(app: &App) -> iced::Element<'_, Message, iced_kit::Theme> {
    /// Settings::<Message>::new(&app.settings)
    ///     .stacked(app.window_width <= STACKED_LAYOUT_MAX_WIDTH)
    ///     .into()
    /// # }
    /// ```
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Converts the panel into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let visible = self.visible_pages();
        let sidebar = self.render_sidebar(&visible);
        let content = self.render_content(&visible);

        if self.stacked {
            return column![
                // `Shrink` rather than a share of the panel: with only a few
                // pages a proportional height would leave a large empty block
                // between the navigation and the page it opens. The cap keeps a
                // long page list from pushing the content off-screen, and the
                // list scrolls inside whatever height it ends up with.
                container(sidebar)
                    .width(Length::Fill)
                    .max_height(STACKED_SIDEBAR_MAX_HEIGHT),
                rule::horizontal(1),
                content,
            ]
            .height(Length::Fill)
            .into();
        }

        row![
            container(sidebar).width(Length::Fixed(self.sidebar_width)),
            rule::vertical(1),
            content,
        ]
        .height(Length::Fill)
        .into()
    }

    /// A task that scrolls the content column to a group.
    ///
    /// The panel cannot see a group's position until iced has laid it out, so
    /// this reads the group's bounds and then scrolls to where it starts.
    ///
    /// Return it from `update` after applying the event that asked for it; see
    /// [`SettingsState`] for the whole sequence. It is a no-op if the group is
    /// not on the page currently drawn.
    pub fn scroll_to_group(page: usize, group: usize) -> Task<Message>
    where
        Message: Send + 'static,
    {
        iced::advanced::widget::operate(super::scroll::scroll_to_group(group_id(page, group)))
    }
}

impl<'a, Message: Clone + 'a> From<Settings<'a, Message>> for Element<'a, Message, Theme> {
    fn from(settings: Settings<'a, Message>) -> Self {
        settings.into_element()
    }
}

/// The identifier the content column's scrollable is given.
///
/// Exposed so a caller can scroll or snap the settings content itself.
#[must_use]
pub fn content_id() -> iced::widget::Id {
    iced::widget::Id::new("iced-kit-settings-content")
}

/// The identifier a group's container is given.
///
/// Exposed so a caller can find a group's bounds, which is what
/// [`Settings::scroll_to_group`] does internally.
#[must_use]
pub fn group_id(page: usize, group: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("iced-kit-settings-group-{page}-{group}"))
}

/// Builds a settings panel.
pub fn settings<'a, Message: Clone + 'a>(state: &'a SettingsState) -> Settings<'a, Message> {
    Settings::new(state)
}

#[cfg(test)]
mod tests {
    use super::{SettingsEvent, SettingsState};

    #[test]
    fn a_new_state_shows_the_first_page_with_no_search() {
        let state = SettingsState::new();

        assert_eq!(state.selected_page(), 0);
        assert_eq!(state.selected_group(), None);
        assert_eq!(state.query(), "");
    }

    #[test]
    fn selecting_a_page_clears_the_selected_group() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectGroup { page: 1, group: 2 });
        assert_eq!(state.selected_group(), Some(2));

        state.apply(SettingsEvent::SelectPage(1));
        assert_eq!(
            state.selected_group(),
            None,
            "a page has no group selected until one is chosen"
        );
    }

    #[test]
    fn selecting_a_group_also_opens_its_page() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectGroup { page: 2, group: 1 });

        assert_eq!(state.selected_page(), 2);
        assert_eq!(state.selected_group(), Some(1));
    }

    #[test]
    fn selecting_a_group_requests_a_scroll_to_it() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectGroup { page: 1, group: 3 });

        assert_eq!(state.take_pending_scroll(), Some((1, 3)));
    }

    #[test]
    fn a_pending_scroll_is_taken_only_once() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectGroup { page: 0, group: 1 });

        assert!(state.take_pending_scroll().is_some());
        assert_eq!(
            state.take_pending_scroll(),
            None,
            "acting on a scroll must not repeat it on the next frame"
        );
    }

    #[test]
    fn selecting_a_page_requests_no_scroll() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectPage(1));

        assert_eq!(state.take_pending_scroll(), None);
    }

    #[test]
    fn searching_keeps_the_selection() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectPage(2));
        state.apply(SettingsEvent::Search("theme".to_owned()));

        assert_eq!(state.query(), "theme");
        assert_eq!(
            state.selected_page(),
            2,
            "a search must not move the user off their page"
        );
    }

    #[test]
    fn select_page_and_set_query_are_reachable_without_events() {
        let mut state = SettingsState::new();

        state.select_page(3);
        state.set_query("dark");

        assert_eq!(state.selected_page(), 3);
        assert_eq!(state.query(), "dark");
    }

    #[test]
    fn group_ids_are_distinct_per_group() {
        use super::group_id;

        assert_ne!(group_id(0, 0), group_id(0, 1));
        assert_ne!(group_id(0, 0), group_id(1, 0));
    }

    #[test]
    fn the_content_id_is_stable() {
        assert_eq!(super::content_id(), super::content_id());
    }
}
