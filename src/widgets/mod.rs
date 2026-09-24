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
pub mod data_table;
pub mod display;
#[cfg(feature = "dock")]
pub mod dock;
pub mod form;
pub mod group_box;
pub mod input;
pub mod list;
pub mod markdown;
pub mod navigation;
pub mod number;
pub mod overlay;
pub mod plot;
pub mod resizable;
pub(crate) mod resize_edge;
pub(crate) mod reveal;
pub mod select;
pub mod sidebar;
pub mod skeleton;
pub mod spinner;
pub mod tabs;
pub mod title_bar;
pub mod toggle;
pub mod typography;
pub mod virtual_list;

pub use avatar::{avatar, avatar_with_label, avatar_with_name, AvatarLabel, AvatarShape};
pub use button::{
    button, icon_button, Button, ButtonGroup, ButtonGroupLayout, DropdownButton, Icon, IconSource,
    LoadingIcon, Toggle, ToggleGroup, ToggleVariant,
};
pub use carousel::{carousel, Carousel, CarouselAxis, CarouselState};
pub use chart::{Chart, ChartKind, Series};
pub use data_table::{Column, DataTable, SortDirection, SortKey, TableState, Width};
pub use display::{alert, badge, card, divider, empty_state, progress, vertical_divider, Tone};
pub use form::{field, form, Field, FieldLabel, Form, FormLabelLayout};
pub use group_box::{group_box, GroupBox, GroupBoxVariant};
pub use input::{
    addon, group_button, group_icon_button, input_group, label, password, text_area, text_input,
    AddonAlignment, InputGroup, InputGroupAddon, TextArea, TextInput,
};
pub use list::{list, tag, ListItem};
pub use markdown::Markdown as MarkdownDocument;
pub use navigation::{
    accordion, accordion_builder, pagination, Accordion, Section as AccordionSection,
};
pub use number::{number_input, otp_input, NumberInput, OtpInput};
pub use overlay::{
    dialog_actions, dialog_close, dialog_description, dialog_title, header_with_icon, tooltip,
    tooltip_at, tooltip_bubble, AlertDialog, AlertTone, ContextMenu, Dialog, DialogButtonProps,
    DialogContent, DialogFooter, DialogHeader, DialogWidth, Drawer, DrawerSide, DrawerSize,
    Dropdown, DropdownAlign, Layer, MenuItem, Modal, Popover, PopoverPlacement, Toast, ToastKind,
    ToastPlacement, Toasts, TooltipPosition,
};
pub use resizable::{PaneGrid, Resizable, SplitAxis};
pub use select::{select, select_fill};
pub use sidebar::{
    sidebar, Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
    SidebarMenuItem, SidebarSide, SidebarToggleButton,
};
pub use skeleton::{skeleton, skeleton_list_item, skeleton_table, SkeletonShape};
pub use spinner::{ring_progress, spinner, spinner_styled, SpinnerStyle};
pub use tabs::{tabs, Tab, TabStrip, TabVariant};
pub use title_bar::{TitleBar, WindowControl};
pub use toggle::{checkbox, radio, slider, switch};
pub use typography::{code, heading, kbd, muted_text, paragraph, shortcut, Heading};
pub use virtual_list::{text_row, virtual_list, VirtualList, VirtualListState};

pub use crate::theme::catalog::ButtonVariant;
