//! The `iced-kit` components.
//!
//! Components are available both as a module and as a constructor function of
//! the same name, matching how `iced::widget` exposes its own widgets:
//!
//! ```
//! # use iced_kit::widgets::button;
//! # #[derive(Clone, Debug)] enum Message { Save }
//! # fn view() -> iced::Element<'static, Message, iced_kit::Theme> {
//! button("Save").primary().on_press(Message::Save).into()
//! # }
//! ```

pub mod avatar;
pub mod button;
pub mod carousel;
pub mod chart;
pub mod chat;
pub mod color_picker;
pub mod combobox;
pub mod data_table;
pub mod date;
pub mod details;
pub mod display;
#[cfg(feature = "dock")]
pub mod dock;
pub mod form;
pub mod group_box;
pub mod input;
pub mod list;
pub mod loading;
pub mod markdown;
pub mod navigation;
pub mod number;
pub mod overlay;
pub mod plot;
pub mod resizable;
pub(crate) mod resize_edge;
pub(crate) mod reveal;
pub mod ribbon;
pub mod select;
pub mod sidebar;
pub mod skeleton;
pub mod spinner;
pub mod tabs;
pub mod title_bar;
pub mod toggle;
pub mod tree;
pub mod typography;
pub mod virtual_list;

pub use avatar::{
    avatar, avatar_group, avatar_with_label, avatar_with_name, Avatar, AvatarGroup, AvatarLabel,
    AvatarShape,
};
pub use button::{
    button, icon_button, Button, ButtonGroup, ButtonGroupLayout, DropdownButton, Icon, IconSource,
    LoadingIcon, Toggle, ToggleGroup, ToggleVariant,
};
pub use carousel::{carousel, Carousel, CarouselAxis, CarouselState};
// The chat family is re-exported whole: its five parts compose into one screen,
// so a caller building that screen wants all of them and its names are already
// namespaced by their own variety (`Bubble`, `Marker`, `Attachment`).
pub use chart::{Chart, ChartKind, Series};
pub use chat::{
    attachment, attachment_group, bubble, marker, message, message_group, message_scroller,
    Attachment, AttachmentActions, AttachmentAxis, AttachmentContent, AttachmentDescription,
    AttachmentGroup, AttachmentMedia, AttachmentStatus, AttachmentTitle, Bubble, BubbleContent,
    BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant, Marker, MarkerLoadingStyle,
    MarkerVariant, Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
    MessageGroup, MessageHeader, MessageScroller, MessageScrollerState,
};
pub use color_picker::{
    color_picker, color_picker_panel, default_swatches, parse_hex, ColorPicker, ColorPickerPanel,
    Hsv,
};
pub use combobox::{
    combobox, combobox_panel, filter_options, ComboBox, ComboBoxOption, ComboBoxPanel,
};
pub use data_table::{Column, ColumnGroup, DataTable, SortDirection, SortKey, TableState, Width};
pub use date::{
    calendar, date_picker, month_grid, month_name, Calendar, Date, DatePicker, DatePreset, Weekday,
};
pub use details::{
    clipboard_button, description_list, link, rating, status_bar, ClipboardButton, Description,
    DescriptionLayout, DescriptionList, DescriptionText, Link, Rating, StatusBar,
};
pub use display::{
    alert, alert_builder, badge, badge_builder, card, divider, empty_state, horizontal_separator,
    progress, vertical_divider, vertical_separator, Alert, Badge, BadgeVariant, Separator,
    SeparatorStyle, Tone,
};
pub use form::{field, form, Field, FieldLabel, Form, FormLabelLayout};
pub use group_box::{group_box, GroupBox, GroupBoxVariant};
pub use input::{
    addon, group_button, group_icon_button, input_group, label, password, text_area, text_input,
    AddonAlignment, InputGroup, InputGroupAddon, TextArea, TextInput,
};
pub use list::{list, tag, ListItem};
pub use loading::{
    collapsible, shimmer, shimmer_block, shimmer_text, Collapsible, Shimmer, ShimmerSpread,
    ShimmerStyle,
};
pub use markdown::Markdown as MarkdownDocument;
pub use navigation::{
    accordion, accordion_builder, app_menu_bar, breadcrumb, pagination, stepper, Accordion, Crumb,
    MenuTitle, Section as AccordionSection, Step, StepLayout, Stepper,
};
pub use number::{number_input, otp_input, NumberInput, OtpInput};
pub use overlay::{
    dialog_actions, dialog_close, dialog_description, dialog_title, header_with_icon,
    popover_dismiss_area, sheet, tooltip, tooltip_at, tooltip_at_with_shortcut, tooltip_bubble,
    tooltip_bubble_with_shortcut, tooltip_with_shortcut, trigger, AlertDialog, AlertTone,
    ContextMenu, Dialog, DialogButtonProps, DialogContent, DialogFooter, DialogHeader, DialogWidth,
    Drawer, DrawerSide, DrawerSize, Dropdown, DropdownAlign, HoverCard, HoverCardPlacement, Layer,
    MenuItem, Modal, Popover, PopoverPlacement, Toast, ToastKind, ToastPlacement, Toasts,
    TooltipPosition, Trigger, SHEET_TOP_INSET,
};
pub use resizable::{PaneGrid, Resizable, SplitAxis};
pub use ribbon::{
    ribbon, CollapseMode, Ribbon, RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool,
};
pub use select::{searchable_select, searchable_select_panel, select, select_fill};
pub use sidebar::{
    sidebar, Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
    SidebarMenuItem, SidebarSide, SidebarToggleButton,
};
pub use skeleton::{skeleton, skeleton_list_item, skeleton_table, SkeletonShape};
pub use spinner::{ring_progress, spinner, spinner_styled, SpinnerStyle};
pub use tabs::{tabs, Tab, TabStrip, TabVariant};
pub use title_bar::{TitleBar, WindowControl};
pub use toggle::{checkbox, radio, slider, switch};
pub use tree::{tree, Tree, TreeEvent, TreeItem, TreeRow, TreeView};
pub use typography::{
    code, heading, kbd, label_builder, muted_text, paragraph, shortcut, Heading, HighlightsMatch,
    Label,
};
pub use virtual_list::{text_row, virtual_list, VirtualList, VirtualListState};

pub use crate::theme::catalog::ButtonVariant;
