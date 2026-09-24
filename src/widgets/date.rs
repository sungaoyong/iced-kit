//! Calendar dates, and the arithmetic a month grid needs.
//!
//! A date here is a plain `(year, month, day)` with no time zone and no clock,
//! which is all a date picker deals in. The crate takes no date-library
//! dependency for it: the calendar rules that matter — month lengths, leap
//! years, the weekday a date falls on — are a few dozen lines of arithmetic
//! that are worth owning outright, since they are the part a picker has to get
//! exactly right.

use std::fmt;

use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// The weekday a zero-based index refers to, Monday first.
    ///
    /// ```
    /// # use iced_kit::widgets::date::Weekday;
    /// assert_eq!(Weekday::from_index(0), Weekday::Monday);
    /// assert_eq!(Weekday::from_index(6), Weekday::Sunday);
    /// // An index past the week wraps rather than failing.
    /// assert_eq!(Weekday::from_index(7), Weekday::Monday);
    /// ```
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        match index % 7 {
            0 => Self::Monday,
            1 => Self::Tuesday,
            2 => Self::Wednesday,
            3 => Self::Thursday,
            4 => Self::Friday,
            5 => Self::Saturday,
            _ => Self::Sunday,
        }
    }

    /// The index of the weekday, Monday being zero.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }

    /// The weekday's short name, for a column heading.
    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Monday => "Mo",
            Self::Tuesday => "Tu",
            Self::Wednesday => "We",
            Self::Thursday => "Th",
            Self::Friday => "Fr",
            Self::Saturday => "Sa",
            Self::Sunday => "Su",
        }
    }
}

/// A calendar date: a year, a month and a day, with no time or zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

impl Date {
    /// Creates a date, or `None` when the parts do not name a real day.
    ///
    /// ```
    /// # use iced_kit::widgets::date::Date;
    /// assert!(Date::from_ymd(2024, 2, 29).is_some(), "2024 is a leap year");
    /// assert!(Date::from_ymd(2023, 2, 29).is_none(), "2023 is not");
    /// assert!(Date::from_ymd(2023, 13, 1).is_none());
    /// ```
    #[must_use]
    pub const fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        if month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) {
            return None;
        }

        Some(Self { year, month, day })
    }

    /// The date's year.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The date's month, 1 to 12.
    #[must_use]
    pub const fn month(self) -> u32 {
        self.month
    }

    /// The date's day of the month, 1 to 31.
    #[must_use]
    pub const fn day(self) -> u32 {
        self.day
    }

    /// The weekday the date falls on.
    ///
    /// Derived from the number of days since a known Thursday (1970-01-01),
    /// which is exact for every date a picker will meet.
    #[must_use]
    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday, i.e. index 3 when Monday is zero.
        let days = self.days_since_epoch();
        let index = (days + 3).rem_euclid(7) as usize;

        Weekday::from_index(index)
    }

    /// Days from 1970-01-01, negative before it.
    ///
    /// The shift keeps the division exact for negative years, which is why the
    /// offset is added before dividing rather than after.
    fn days_since_epoch(self) -> i64 {
        let year = i64::from(self.year);
        let day = i64::from(self.day);

        // Whole years before this one, leap days included, then the whole
        // months of this year before this one.
        let mut days = 365 * (year - 1970) + leap_days_before(self.year) - leap_days_before(1970);

        for month in 1..self.month {
            days += i64::from(days_in_month(self.year, month));
        }

        days + day - 1
    }

    /// Adds days, rolling over months and years.
    #[must_use]
    pub fn add_days(self, days: i64) -> Self {
        let total = self.days_since_epoch() + days;
        Self::from_days_since_epoch(total)
    }

    /// The date `months` months away, clamped to the last day of the target
    /// month.
    ///
    /// The clamp is what makes "one month after the 31st" mean the end of the
    /// next month rather than an invalid date or a silent skip backwards.
    #[must_use]
    pub fn add_months(self, months: i32) -> Self {
        let index = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(months);
        let year = (index.div_euclid(12)) as i32;
        let month = (index.rem_euclid(12) + 1) as u32;
        let day = self.day.min(days_in_month(year, month));

        Self::from_ymd(year, month, day).unwrap_or(self)
    }

    /// The first day of the date's month.
    #[must_use]
    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// How many days the date's month has.
    #[must_use]
    pub const fn days_in_its_month(self) -> u32 {
        days_in_month(self.year, self.month)
    }

    fn from_days_since_epoch(days: i64) -> Self {
        // Walk the years, then the months. A picker's range is small enough that
        // a loop is clearer than the closed-form conversion, and it cannot
        // drift by a day the way an approximation can.
        let mut remaining = days;
        let mut year = 1970i32;

        loop {
            let length = i64::from(days_in_year(year));
            if remaining >= length {
                remaining -= length;
                year += 1;
            } else if remaining < 0 {
                year -= 1;
                remaining += i64::from(days_in_year(year));
            } else {
                break;
            }
        }

        let mut month = 1u32;
        loop {
            let length = i64::from(days_in_month(year, month));
            if remaining >= length {
                remaining -= length;
                month += 1;
            } else {
                break;
            }
        }

        Self {
            year,
            month,
            day: (remaining + 1) as u32,
        }
    }
}

impl fmt::Display for Date {
    /// The date as `YYYY-MM-DD`, which is what a field shows and what
    /// [`parse`] reads back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Parses a date from `YYYY-MM-DD`.
///
/// Returns `None` for anything else, including a date that does not exist.
///
/// ```
/// # use iced_kit::widgets::date::{parse, Date};
/// assert_eq!(parse("2024-02-29"), Date::from_ymd(2024, 2, 29));
/// assert_eq!(parse("2023-02-29"), None);
/// assert_eq!(parse("not a date"), None);
/// ```
#[must_use]
pub fn parse(text: &str) -> Option<Date> {
    let text = text.trim();
    let mut parts = text.split('-');

    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Date::from_ymd(year, month, day)
}

/// Whether `year` is a leap year, by the Gregorian rule.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days `month` has in `year`.
#[must_use]
pub const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        // An out-of-range month has no days; callers that validate first never
        // reach this, and a `0` is a safer answer than a panic in a widget.
        _ => 0,
    }
}

/// How many days `year` has.
#[must_use]
pub const fn days_in_year(year: i32) -> u32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// How many leap days fall before `year`'s January.
///
/// Counts the years divisible by four, less the centuries, plus the
/// four-centuries back — the standard inclusion-exclusion for the Gregorian
/// rule.
fn leap_days_before(year: i32) -> i64 {
    let year = i64::from(year) - 1;

    year / 4 - year / 100 + year / 400
}

/// The weeks of `month`, each seven days, with the days around it filled in
/// from the neighbouring months.
///
/// The grid is what a calendar draws: leading days from the previous month and
/// trailing days from the next, so every row has seven cells and the month
/// reads as a block.
///
/// ```
/// # use iced_kit::widgets::date::{month_grid, Weekday, Date};
/// let weeks = month_grid(2024, 2, Weekday::Monday);
/// assert_eq!(weeks.len(), 5);
/// assert!(weeks.iter().all(|week| week.len() == 7));
/// // The grid starts on the Monday on or before the 1st.
/// assert_eq!(weeks[0][0].weekday(), Weekday::Monday);
/// # let _ = Date::from_ymd(2024, 1, 1);
/// ```
#[must_use]
pub fn month_grid(year: i32, month: u32, first_day: Weekday) -> Vec<Vec<Date>> {
    let Some(first) = Date::from_ymd(year, month, 1) else {
        return Vec::new();
    };

    // How many days of the previous month lead this one: the offset from the
    // week's chosen first day back to the 1st.
    let lead = (first.weekday().index() + 7 - first_day.index()) % 7;
    let total = lead + days_in_month(year, month) as usize;
    // Always whole weeks, so a month that spills into a sixth row still shows
    // its trailing days rather than dropping them.
    let weeks = total.div_ceil(7);
    // A month grid is at most six weeks, so every offset here fits an i64 with
    // room to spare; the conversion is spelled out rather than cast.
    let lead = i64::try_from(lead).unwrap_or(0);
    let start = first.add_days(-lead);

    (0..weeks)
        .map(|week| {
            (0..7)
                .map(|day| {
                    let offset = i64::try_from(week * 7 + day).unwrap_or(i64::MAX);
                    start.add_days(offset)
                })
                .collect()
        })
        .collect()
}

/// What a [`date_picker`] shows on one of its quick-choice buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a DatePreset does nothing unless it is given to a DatePicker"]
pub struct DatePreset {
    label: String,
    range: (Date, Date),
}

impl DatePreset {
    /// A preset that picks a single date.
    pub fn single(label: impl Into<String>, date: Date) -> Self {
        Self {
            label: label.into(),
            range: (date, date),
        }
    }

    /// A preset that picks a span.
    pub fn spanning(label: impl Into<String>, start: Date, end: Date) -> Self {
        Self {
            label: label.into(),
            range: if start <= end {
                (start, end)
            } else {
                (end, start)
            },
        }
    }

    /// The preset's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The span the preset selects.
    #[must_use]
    pub const fn span(&self) -> (Date, Date) {
        self.range
    }
}

/// A month grid, with a heading and a weekday header row.
///
/// The calendar draws one or more months and reports a date when one is
/// clicked. It is stateless: the caller holds the visible month and the
/// selection, and decides what a picked date means.
///
/// ```
/// # use iced_kit::widgets::{calendar, date::{Date, Weekday}};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Picked(Date) }
/// # fn view(today: Date, selected: Option<Date>) -> Element<'static, Message, Theme> {
/// calendar(today, selected).on_select(Message::Picked).into()
/// # }
/// ```
#[must_use = "a Calendar does nothing unless it is turned into an Element"]
pub struct Calendar<'a, Message> {
    month: Date,
    selected: Option<Date>,
    range: Option<(Date, Date)>,
    first_day: Weekday,
    number_of_months: usize,
    on_select: Option<Box<dyn Fn(Date) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Calendar<'a, Message> {
    /// Creates a calendar showing `month`'s month.
    pub fn new(month: Date, selected: Option<Date>) -> Self {
        Self {
            month: month.first_of_month(),
            selected,
            range: None,
            first_day: Weekday::Monday,
            number_of_months: 1,
            on_select: None,
        }
    }

    /// Draws several months starting at the current one.
    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count.clamp(1, 3);
        self
    }

    /// Sets which weekday the columns start on. The default is Monday.
    pub fn first_day_of_week(mut self, day: Weekday) -> Self {
        self.first_day = day;
        self
    }

    /// Marks a span of days, for a range picker.
    pub fn range(mut self, range: (Date, Date)) -> Self {
        let (start, end) = range;
        self.range = Some(if start <= end {
            (start, end)
        } else {
            (end, start)
        });
        self
    }

    /// Reports a clicked day.
    pub fn on_select(mut self, on_select: impl Fn(Date) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Whether `date` is drawn as selected, by the selection or by the range.
    #[must_use]
    pub fn is_selected(&self, date: Date) -> bool {
        if self.selected == Some(date) {
            return true;
        }

        self.range
            .is_some_and(|(start, end)| date >= start && date <= end)
    }

    /// The months this calendar draws, in order.
    #[must_use]
    pub fn months(&self) -> Vec<Date> {
        (0..self.number_of_months)
            .map(|offset| {
                let months = i32::try_from(offset).unwrap_or(0);
                self.month.add_months(months)
            })
            .collect()
    }

    /// Turns the calendar into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let mut months = column![].spacing(16);

        for month in self.months() {
            months = months.push(self.month_block(month));
        }

        months.into()
    }

    /// Draws one month: its heading, the weekday header and its weeks.
    fn month_block(&self, month: Date) -> Element<'a, Message, Theme> {
        let heading_style = Size::Sm.text();
        let day_style = Size::Xs.text();
        let grid = month_grid(month.year(), month.month(), self.first_day);

        // The header row names the columns in the order they are drawn, so a
        // calendar whose week starts on Sunday does not mislabel every column.
        let mut header = row![].spacing(2);
        for column in 0..7 {
            let weekday = Weekday::from_index(self.first_day.index() + column);
            header = header.push(
                container(
                    text(weekday.short_name())
                        .size(day_style.size)
                        .line_height(day_style.line_height()),
                )
                .width(Length::Fill)
                .center_x(Length::Fill)
                .class(Box::new(|theme: &Theme| container::Style {
                    text_color: Some(theme.colors().muted_foreground),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>),
            );
        }

        let mut body = column![header].spacing(2);

        for week in grid {
            let mut row_widget = row![].spacing(2);

            for date in week {
                row_widget = row_widget.push(self.day_cell(date, month, day_style.size));
            }

            body = body.push(row_widget);
        }

        let month_label = MONTH_NAMES
            .get(month.month().saturating_sub(1) as usize)
            .copied()
            .unwrap_or("");

        column![
            text(format!("{month_label} {}", month.year()))
                .size(heading_style.size)
                .line_height(heading_style.line_height()),
            body,
        ]
        .spacing(8)
        .into()
    }

    /// Draws one day: its number, marked when it is inside the month, selected
    /// or outside the picker's range.
    fn day_cell(&self, date: Date, month: Date, text_size: f32) -> Element<'a, Message, Theme> {
        let in_month = date.month() == month.month() && date.year() == month.year();
        let selected = self.is_selected(date);
        let line_height = Size::Xs.text().line_height();

        let mut cell = button(
            container(
                text(format!("{}", date.day()))
                    .size(text_size)
                    .line_height(line_height),
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
        )
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fixed(28.0))
        .class(Box::new(move |theme: &Theme, status| {
            day_cell_style(theme, status, in_month, selected)
        }) as button::StyleFn<'a, Theme>);

        if let Some(on_select) = self.on_select.as_ref() {
            cell = cell.on_press(on_select(date));
        }

        cell.into()
    }
}

impl<'a, Message: Clone + 'a> From<Calendar<'a, Message>> for Element<'a, Message, Theme> {
    fn from(calendar: Calendar<'a, Message>) -> Self {
        calendar.into_element()
    }
}

/// The month names, January first.
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The name of a month, 1 to 12.
#[must_use]
pub fn month_name(month: u32) -> &'static str {
    // `saturating_sub` would turn month 0 into index 0, naming January for a
    // month that does not exist; `checked_sub` refuses instead.
    month
        .checked_sub(1)
        .and_then(|index| MONTH_NAMES.get(index as usize))
        .copied()
        .unwrap_or("")
}

/// Builds a month grid.
pub fn calendar<'a, Message: Clone + 'a>(
    month: Date,
    selected: Option<Date>,
) -> Calendar<'a, Message> {
    Calendar::new(month, selected)
}

/// The appearance of one day of a calendar.
fn day_cell_style(
    theme: &Theme,
    status: button::Status,
    in_month: bool,
    selected: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    let text_color = if selected {
        colors.background
    } else if in_month {
        colors.foreground
    } else {
        // A day from a neighbouring month is dimmed rather than hidden: it is
        // the cell that makes a week whole, but it is not this month's.
        Color {
            a: colors.muted_foreground.a * 0.5,
            ..colors.muted_foreground
        }
    };

    button::Style {
        background: if selected {
            Some(iced::Background::Color(colors.primary))
        } else if hovered {
            Some(iced::Background::Color(colors.accent))
        } else {
            None
        },
        text_color,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().sm).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// A date field: a text field showing `YYYY-MM-DD`, with a calendar in a popup.
///
/// Like every iced overlay in this crate, the field and the calendar are two
/// pieces: this builds the field, and the application draws a [`Calendar`]
/// through [`Layer`](crate::widgets::overlay::Layer) when the field is open.
/// The application also owns the calendar's month, because that is view state.
///
/// ```
/// # use iced_kit::widgets::{date_picker, date::{Date, Weekday}};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Typed(String), Toggled }
/// # fn view<'a>(text: &'a str, open: bool, today: Date) -> Element<'a, Message, Theme> {
/// date_picker("Pick a date", text)
///     .open(open)
///     .on_input(Message::Typed)
///     .on_toggle(Message::Toggled)
///     .into()
/// # }
/// ```
#[must_use = "a DatePicker does nothing unless it is turned into an Element"]
pub struct DatePicker<'a, Message> {
    placeholder: String,
    text: String,
    open: bool,
    clearable: Option<Message>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_toggle: Option<Message>,
    fill: bool,
}

impl<'a, Message: Clone + 'a> DatePicker<'a, Message> {
    /// Creates a field over `text`, which is the date as the user typed it.
    pub fn new(placeholder: impl Into<String>, text: &str) -> Self {
        Self {
            placeholder: placeholder.into(),
            text: text.to_owned(),
            open: false,
            clearable: None,
            on_input: None,
            on_toggle: None,
            fill: false,
        }
    }

    /// Marks the field as open, which keeps it outlined while the calendar
    /// shows.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Makes the field editable, reporting each change.
    ///
    /// A date field that can be typed into needs the text even when it does not
    /// parse, so the message carries the raw string.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Reports a press on the calendar button.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Shows a clear button that emits this message.
    pub fn clearable(mut self, message: Message) -> Self {
        self.clearable = Some(message);
        self
    }

    /// Stretches the field to its container's width.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// The date the field currently reads, if the text is a valid date.
    #[must_use]
    pub fn date(&self) -> Option<Date> {
        parse(&self.text)
    }

    /// Turns the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let height = Size::Md.height();

        let mut content = row![].spacing(8).align_y(Alignment::Center);

        let field: Element<'a, Message, Theme> = if let Some(on_input) = self.on_input {
            crate::widgets::input::text_input::<Message>(&self.placeholder, &self.text)
                .on_input(on_input)
                .into()
        } else {
            {
                let style = Size::Md.text();
                let empty = self.text.is_empty();

                text(if empty {
                    self.placeholder.clone()
                } else {
                    self.text.clone()
                })
                .size(style.size)
                .line_height(iced::Pixels(height.max(f32::from(style.line_height()))))
                .class(Box::new(move |theme: &Theme| text::Style {
                    color: Some(if empty {
                        theme.colors().muted_foreground
                    } else {
                        theme.colors().foreground
                    }),
                }) as text::StyleFn<'a, Theme>)
                .into()
            }
        };

        content = content.push(container(field).width(Length::Fill));

        if let Some(message) = self.clearable {
            if !self.text.is_empty() {
                content = content.push(
                    crate::widgets::icon_button::<Message>()
                        .icon(crate::icons::IconName::X)
                        .ghost()
                        .size(Size::Sm)
                        .on_press(message),
                );
            }
        }

        content = content.push(
            crate::widgets::Icon::new(crate::icons::IconName::Calendar).into_element(Size::Sm),
        );

        let mut trigger = button(content)
            .padding(Padding {
                top: 0.0,
                right: 8.0,
                bottom: 0.0,
                left: Size::Md.padding(),
            })
            .height(Length::Fixed(height))
            .width(if self.fill {
                Length::Fill
            } else {
                Length::Shrink
            })
            .class(
                Box::new(move |theme: &Theme, status| date_field_style(theme, status, self.open))
                    as button::StyleFn<'a, Theme>,
            );

        if let Some(message) = self.on_toggle {
            trigger = trigger.on_press(message);
        }

        trigger.into()
    }
}

impl<'a, Message: Clone + 'a> From<DatePicker<'a, Message>> for Element<'a, Message, Theme> {
    fn from(picker: DatePicker<'a, Message>) -> Self {
        picker.into_element()
    }
}

/// Builds a date field.
pub fn date_picker<'a, Message: Clone + 'a>(
    placeholder: impl Into<String>,
    text: &str,
) -> DatePicker<'a, Message> {
    DatePicker::new(placeholder, text)
}

/// The appearance of a date field.
fn date_field_style(theme: &Theme, status: button::Status, open: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: Some(iced::Background::Color(if hovered && !open {
            colors.accent
        } else {
            colors.surface
        })),
        text_color: colors.foreground,
        border: iced::Border {
            color: if open { colors.primary } else { colors.border },
            width: 1.0,
            radius: f32::from(theme.radius().md).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{days_in_month, days_in_year, is_leap_year, month_grid, parse, Date, Weekday};

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000), "divisible by 400");
        assert!(!is_leap_year(1900), "a century, but not by 400");
        assert!(!is_leap_year(2023));
        assert_eq!(days_in_year(2024), 366);
        assert_eq!(days_in_year(2023), 365);
    }

    #[test]
    fn month_lengths_are_right_in_every_month() {
        let expected = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for (index, days) in expected.iter().enumerate() {
            let month = u32::try_from(index + 1).expect("a month number");
            assert_eq!(days_in_month(2023, month), *days, "month {month} of 2023");
        }

        assert_eq!(days_in_month(2024, 2), 29, "February in a leap year");
        assert_eq!(days_in_month(4, 2), 29, "the year 4 was a leap year");
        assert_eq!(days_in_month(2, 13), 0, "there is no month 13");
    }

    #[test]
    fn a_date_rejects_parts_that_do_not_name_a_day() {
        assert!(Date::from_ymd(2024, 2, 29).is_some());
        assert!(Date::from_ymd(2023, 2, 29).is_none());
        assert!(Date::from_ymd(2024, 0, 1).is_none());
        assert!(Date::from_ymd(2024, 13, 1).is_none());
        assert!(Date::from_ymd(2024, 1, 0).is_none());
        assert!(Date::from_ymd(2024, 1, 32).is_none());
    }

    /// The weekday is the piece that is easy to get subtly wrong, so it is
    /// checked against dates whose weekday is independently known.
    #[test]
    fn weekdays_are_correct_for_known_dates() {
        for (year, month, day, expected) in [
            (1970, 1, 1, Weekday::Thursday),
            (2000, 1, 1, Weekday::Saturday),
            (2024, 1, 1, Weekday::Monday),
            (2024, 2, 29, Weekday::Thursday),
            (2023, 12, 25, Weekday::Monday),
            (1969, 12, 31, Weekday::Wednesday),
        ] {
            let date = Date::from_ymd(year, month, day).expect("a real date");
            assert_eq!(date.weekday(), expected, "{date} falls on {expected:?}");
        }
    }

    #[test]
    fn a_week_of_days_advances_one_weekday_at_a_time() {
        // 2024-03-01 is a Friday, so the seven days from it visit every
        // weekday exactly once and return to Friday.
        let start = Date::from_ymd(2024, 3, 1).expect("a real date");

        for offset in 0..7 {
            let date = start.add_days(offset);
            let expected = Weekday::from_index((Weekday::Friday.index() + offset as usize) % 7);
            assert_eq!(date.weekday(), expected, "{date}");
        }

        assert_eq!(
            start.add_days(7).weekday(),
            start.weekday(),
            "a week later is the same weekday"
        );
    }

    #[test]
    fn adding_days_rolls_over_months_and_years() {
        let date = Date::from_ymd(2023, 12, 31).expect("a real date");

        assert_eq!(date.add_days(1), Date::from_ymd(2024, 1, 1).unwrap());
        assert_eq!(date.add_days(-1), Date::from_ymd(2023, 12, 30).unwrap());
        assert_eq!(date.add_days(366), Date::from_ymd(2024, 12, 31).unwrap());
    }

    #[test]
    fn walking_a_year_forward_and_back_returns_to_the_start() {
        let start = Date::from_ymd(2024, 2, 29).expect("a leap day");

        assert_eq!(start.add_days(365).add_days(-365), start);
        assert_eq!(start.add_days(1).add_days(-1), start);
    }

    #[test]
    fn adding_months_clamps_to_the_end_of_the_target_month() {
        let end_of_january = Date::from_ymd(2024, 1, 31).expect("a real date");

        assert_eq!(
            end_of_january.add_months(1),
            Date::from_ymd(2024, 2, 29).unwrap(),
            "February has no 31st, so the date lands on its last day"
        );
        assert_eq!(
            end_of_january.add_months(2),
            Date::from_ymd(2024, 3, 31).unwrap()
        );
    }

    #[test]
    fn adding_months_rolls_over_years_in_both_directions() {
        let date = Date::from_ymd(2024, 6, 15).expect("a real date");

        assert_eq!(date.add_months(7), Date::from_ymd(2025, 1, 15).unwrap());
        assert_eq!(date.add_months(-6), Date::from_ymd(2023, 12, 15).unwrap());
        assert_eq!(date.add_months(12), Date::from_ymd(2025, 6, 15).unwrap());
        assert_eq!(date.add_months(-24), Date::from_ymd(2022, 6, 15).unwrap());
        assert_eq!(date.add_months(0), date);
    }

    #[test]
    fn a_month_grid_has_seven_cells_a_week_and_only_whole_weeks() {
        for (year, month) in [(2024, 1), (2024, 2), (2023, 2), (2024, 9), (2024, 12)] {
            let grid = month_grid(year, month, Weekday::Monday);

            assert!(!grid.is_empty(), "{year}-{month}");
            assert!(
                grid.iter().all(|week| week.len() == 7),
                "{year}-{month} has a short week"
            );
            assert!(
                (4..=6).contains(&grid.len()),
                "{year}-{month} has {} weeks",
                grid.len()
            );
        }
    }

    #[test]
    fn a_month_grid_starts_on_the_chosen_first_day() {
        for first_day in [Weekday::Monday, Weekday::Sunday, Weekday::Saturday] {
            let grid = month_grid(2024, 2, first_day);
            let start = grid[0][0];

            assert_eq!(
                start.weekday(),
                first_day,
                "the grid must start on {first_day:?}"
            );
        }
    }

    #[test]
    fn a_month_grid_contains_every_day_of_the_month_in_order() {
        let grid = month_grid(2024, 2, Weekday::Monday);
        let days: Vec<Date> = grid.iter().flatten().copied().collect();

        // Every day of February is present exactly once.
        for day in 1..=29 {
            let date = Date::from_ymd(2024, 2, day).expect("a real date");
            assert_eq!(days.iter().filter(|d| **d == date).count(), 1, "{date}");
        }

        // And the days run consecutively.
        for pair in days.windows(2) {
            assert_eq!(pair[0].add_days(1), pair[1]);
        }
    }

    #[test]
    fn a_month_grid_has_no_weeks_for_an_impossible_month() {
        assert!(month_grid(2024, 13, Weekday::Monday).is_empty());
    }

    #[test]
    fn dates_round_trip_through_their_text_form() {
        for (year, month, day) in [(2024, 2, 29), (1999, 12, 31), (2000, 1, 1), (5, 6, 7)] {
            let date = Date::from_ymd(year, month, day).expect("a real date");
            let text = date.to_string();

            assert_eq!(parse(&text), Some(date), "{text}");
        }
    }

    #[test]
    fn parsing_rejects_anything_that_is_not_a_date() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("2024"), None);
        assert_eq!(parse("2024-02"), None);
        assert_eq!(parse("2024-02-29"), Date::from_ymd(2024, 2, 29));
        assert_eq!(parse("2023-02-29"), None, "a day that does not exist");
        assert_eq!(parse("2024-13-01"), None);
        assert_eq!(parse("2024-02-30"), None);
        assert_eq!(parse(" 2024-02-29 "), Date::from_ymd(2024, 2, 29));
        assert_eq!(parse("2024-2-9"), Date::from_ymd(2024, 2, 9));
        assert_eq!(parse("yesterday"), None);
        assert_eq!(parse("2024-02-29-01"), None);
    }

    #[test]
    fn a_first_of_month_keeps_its_month() {
        let date = Date::from_ymd(2024, 7, 19).expect("a real date");
        assert_eq!(date.first_of_month(), Date::from_ymd(2024, 7, 1).unwrap());
        assert_eq!(date.days_in_its_month(), 31);
    }
}

#[cfg(test)]
mod widget_tests {
    use super::{calendar, date_picker, month_name, Date, DatePreset, Weekday};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked(Date),
        Typed(String),
        Toggled,
        Cleared,
    }

    fn some_date() -> Date {
        Date::from_ymd(2024, 2, 29).expect("a real date")
    }

    #[test]
    fn a_calendar_marks_the_selection_and_the_range() {
        let start = Date::from_ymd(2024, 2, 10).expect("a real date");
        let end = Date::from_ymd(2024, 2, 20).expect("a real date");

        let plain: super::Calendar<'_, Message> = calendar(some_date(), Some(start));
        assert!(plain.is_selected(start));
        assert!(!plain.is_selected(end));

        let ranged: super::Calendar<'_, Message> = calendar(some_date(), None).range((start, end));
        assert!(ranged.is_selected(start), "the range includes its start");
        assert!(ranged.is_selected(end), "and its end");
        assert!(ranged.is_selected(Date::from_ymd(2024, 2, 15).unwrap()));
        assert!(!ranged.is_selected(Date::from_ymd(2024, 3, 1).unwrap()));
    }

    #[test]
    fn a_calendar_normalizes_a_reversed_range() {
        let start = Date::from_ymd(2024, 2, 20).expect("a real date");
        let end = Date::from_ymd(2024, 2, 10).expect("a real date");

        let calendar: super::Calendar<'_, Message> =
            calendar(some_date(), None).range((start, end));
        assert_eq!(calendar.range, Some((end, start)));
    }

    #[test]
    fn a_calendar_draws_the_months_it_was_asked_for() {
        let one: super::Calendar<'_, Message> = calendar(some_date(), None);
        assert_eq!(one.months().len(), 1);

        let three: super::Calendar<'_, Message> = calendar(some_date(), None).number_of_months(3);
        assert_eq!(three.months().len(), 3);
        assert_eq!(three.months()[1], Date::from_ymd(2024, 3, 1).unwrap());
        assert_eq!(three.months()[2], Date::from_ymd(2024, 4, 1).unwrap());

        // A count outside the sensible range is clamped rather than drawn.
        assert_eq!(
            calendar::<Message>(some_date(), None)
                .number_of_months(0)
                .months()
                .len(),
            1
        );
        assert_eq!(
            calendar::<Message>(some_date(), None)
                .number_of_months(99)
                .months()
                .len(),
            3
        );
    }

    #[test]
    fn a_calendar_starts_its_month_on_the_first() {
        let mid_month = Date::from_ymd(2024, 2, 19).expect("a real date");
        let calendar: super::Calendar<'_, Message> = calendar(mid_month, None);

        assert_eq!(calendar.month, Date::from_ymd(2024, 2, 1).unwrap());
    }

    #[test]
    fn calendars_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            calendar::<Message>(some_date(), None).into(),
            calendar::<Message>(some_date(), Some(some_date()))
                .on_select(Message::Picked)
                .into(),
            calendar::<Message>(some_date(), None)
                .number_of_months(2)
                .first_day_of_week(Weekday::Sunday)
                .range((
                    Date::from_ymd(2024, 2, 1).unwrap(),
                    Date::from_ymd(2024, 2, 29).unwrap(),
                ))
                .on_select(Message::Picked)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn month_names_cover_the_year() {
        assert_eq!(month_name(1), "January");
        assert_eq!(month_name(12), "December");
        // Out of range names nothing rather than panicking.
        assert_eq!(month_name(0), "");
        assert_eq!(month_name(13), "");
    }

    #[test]
    fn a_date_preset_records_its_span() {
        let one = some_date();
        let single = DatePreset::single("Today", one);

        assert_eq!(single.label(), "Today");
        assert_eq!(single.span(), (one, one));

        let other = Date::from_ymd(2024, 3, 1).expect("a real date");
        let range = DatePreset::spanning("This month", other, one);
        assert_eq!(range.span(), (one, other), "the range reads earliest first");
    }

    #[test]
    fn a_date_picker_reads_the_text_it_shows() {
        let valid: super::DatePicker<'_, Message> = date_picker("Pick", "2024-02-29");
        assert_eq!(valid.date(), Some(some_date()));

        let half_typed: super::DatePicker<'_, Message> = date_picker("Pick", "2024-02");
        assert_eq!(half_typed.date(), None);

        let empty: super::DatePicker<'_, Message> = date_picker("Pick", "");
        assert_eq!(empty.date(), None);
    }

    #[test]
    fn date_pickers_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            date_picker::<Message>("Pick a date", "").into(),
            date_picker::<Message>("Pick a date", "2024-02-29")
                .open(true)
                .on_toggle(Message::Toggled)
                .clearable(Message::Cleared)
                .fill(true)
                .into(),
            date_picker::<Message>("Pick a date", "2024-02-29")
                .on_input(Message::Typed)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn an_open_date_field_is_outlined_in_the_primary_color() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let open = super::date_field_style(&theme, Status::Active, true);
        let closed = super::date_field_style(&theme, Status::Active, false);

        assert_eq!(open.border.color, theme.colors().primary);
        assert_eq!(closed.border.color, theme.colors().border);
    }

    #[test]
    fn a_selected_day_is_drawn_on_the_primary_fill() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::day_cell_style(&theme, Status::Active, true, true);
        let ordinary = super::day_cell_style(&theme, Status::Active, true, false);
        let outside = super::day_cell_style(&theme, Status::Active, false, false);

        assert_eq!(
            selected.background,
            Some(iced::Background::Color(theme.colors().primary))
        );
        assert!(ordinary.background.is_none());
        assert!(
            outside.text_color.a < ordinary.text_color.a,
            "a day from another month must be dimmer"
        );
    }
}
