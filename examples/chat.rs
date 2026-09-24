//! A chat transcript built from the `chat` family: a virtualized, tail-
//! following [`MessageScroller`] of bubbles and messages, with markers,
//! attachments, emoji reactions, a live composer, and history loading.
//!
//! Run with `cargo run --example chat`.

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length, Task};
use iced_kit::theme::Theme;
use iced_kit::widgets::avatar;
use iced_kit::widgets::chat::{
    attachment, bubble, marker, AttachmentStatus, BubbleReactions, BubbleVariant,
    Message as ChatMessage, MessageAlignment, MessageContent, MessageScroller, MessageScrollerState,
    MarkerLoadingStyle, MarkerVariant,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

/// Who sent a line; drives alignment and bubble colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Author {
    Me,
    Bot,
}

/// One transcript line's data. The scroller renders these into chat widgets.
#[derive(Debug, Clone)]
struct Line {
    author: Author,
    text: String,
    footer: String,
    attachment: Option<AttachmentStatus>,
}

/// The example's application message.
#[derive(Debug, Clone)]
enum Message {
    Noop,
    ToggleTheme,
    Edit(String),
    Send,
    LoadOlder,
    Scrolled(MessageScrollerState),
    JumpToBottom,
}

/// The example's state.
#[derive(Debug)]
struct App {
    dark: bool,
    draft: String,
    messages: Vec<Line>,
    scroller: MessageScrollerState,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let messages = seed(200);
        let scroller = MessageScrollerState::new(messages.len());
        (
            Self {
                dark: false,
                draft: String::new(),
                messages,
                scroller,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        format!("iced-kit chat — {} messages", self.messages.len())
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => {}
            Message::ToggleTheme => self.dark = !self.dark,
            Message::Edit(value) => self.draft = value,
            Message::Send => {
                if !self.draft.is_empty() {
                    self.messages.push(Line {
                        author: Author::Me,
                        text: std::mem::take(&mut self.draft),
                        footer: "now".to_owned(),
                        attachment: None,
                    });
                    // Appending while pinned to the tail keeps it pinned.
                    self.scroller.record_len(self.messages.len());
                }
            }
            Message::LoadOlder => {
                let mut older = seed_older(20, self.messages.len());
                older.append(&mut self.messages);
                self.messages = older;
                // Prepending history leaves the offset alone so the view
                // does not jump away from what the reader was looking at.
                self.scroller.prepend(20);
            }
            Message::Scrolled(state) => self.scroller = state,
            Message::JumpToBottom => self.scroller.pin_to_tail(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let fade = if self.dark {
            Color::from_rgb(0.16, 0.16, 0.18)
        } else {
            Color::from_rgb(0.97, 0.97, 0.98)
        };

        let scroller = MessageScroller::new(&self.messages, &self.scroller, line_row)
            .row_height(116.0)
            .jump_button(true)
            .with_bottom_fade(Some(fade))
            .on_scroll(Message::Scrolled)
            .on_jump_to_bottom(Message::JumpToBottom);

        let header = row![
            text("Chat family").size(22),
            container(button(text(if self.dark { "Light" } else { "Dark" }))
                .on_press(Message::ToggleTheme))
                .width(Length::Fill)
                .align_x(Alignment::End),
        ]
        .align_y(Alignment::Center);

        let composer = row![
            text_input("Type a message…", &self.draft)
                .on_input(Message::Edit)
                .on_submit(Message::Send),
            button(text("Send")).on_press(Message::Send),
            button(text("Load older")).on_press(Message::LoadOlder),
        ]
        .spacing(8);

        let content = column![
            header,
            self.showcase(),
            container(scroller).height(Length::Fill),
            composer,
        ]
        .spacing(12)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .into()
    }

    /// A static strip demonstrating the remaining members of the family: a
    /// separator marker, a loading marker, every bubble variant, and every
    /// attachment status.
    fn showcase<'a>(&self) -> Element<'a, Message, Theme> {
        let _ = self.dark;
        let separator: Element<'a, Message, Theme> =
            marker("Today").with_variant(MarkerVariant::Separator).into();
        let loading: Element<'a, Message, Theme> = marker("Assistant is thinking…")
            .loading(true)
            .with_loading_style(MarkerLoadingStyle::Spinner)
            .into();

        let bubbles: Vec<Element<'a, Message, Theme>> = [
            BubbleVariant::Filled,
            BubbleVariant::Secondary,
            BubbleVariant::Muted,
            BubbleVariant::Tinted,
            BubbleVariant::Outline,
            BubbleVariant::Ghost,
        ]
        .into_iter()
        .map(|variant| bubble("Sample bubble").with_variant(variant).max_width(160.0).into())
        .collect();

        let cards: Vec<Element<'a, Message, Theme>> = [
            AttachmentStatus::Pending,
            AttachmentStatus::Uploading,
            AttachmentStatus::Processing,
            AttachmentStatus::Failed,
            AttachmentStatus::Complete,
        ]
        .into_iter()
        .map(|status| attachment("report.pdf").status(status).into())
        .collect();

        column![
            separator,
            loading,
            row(bubbles).spacing(8),
            row(cards).spacing(8),
        ]
        .spacing(10)
        .into()
    }
}

/// Builds one transcript row as a [`ChatMessage`] with a [`Bubble`] body.
fn line_row(line: &Line, index: usize) -> Element<'_, Message, Theme> {
    let alignment = match line.author {
        Author::Me => MessageAlignment::End,
        Author::Bot => MessageAlignment::Start,
    };
    let variant = match line.author {
        Author::Me => BubbleVariant::Filled,
        Author::Bot => BubbleVariant::Secondary,
    };

    let mut body = bubble(line.text.clone())
        .with_variant(variant)
        .alignment(alignment);

    // Give a few bubbles a reaction chip to show the reactions slot.
    if index % 7 == 3 {
        let chip = button(text("👍").size(14)).on_press(Message::Noop);
        body = body.reactions(BubbleReactions::new().action(chip));
    }

    let mut content = MessageContent::new().bubble(body);
    if let Some(status) = line.attachment {
        let card = attachment(format!("file-{index}.pdf"))
            .status(status)
            .on_click(|| Message::Noop);
        content = content.child(card);
    }

    let name = match line.author {
        Author::Me => "You",
        Author::Bot => "Assistant",
    };
    let mut message = ChatMessage::new()
        .alignment(alignment)
        .content(content)
        .header(name)
        .footer(line.footer.clone());
    if line.author == Author::Bot {
        message = message.avatar(avatar("AI", 30));
    }
    // The scroller gives every row a fixed cell; centring the message in it
    // keeps the bubbles evenly spaced instead of hugging the top edge.
    container(message)
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .padding([6.0, 6.0])
        .into()
}

/// The initial transcript, sized to make virtualization meaningful.
fn seed(count: usize) -> Vec<Line> {
    (0..count)
        .map(|i| Line {
            author: if i % 2 == 0 { Author::Bot } else { Author::Me },
            text: format!("Message #{i} — the quick brown fox jumps over the lazy dog."),
            footer: format!("{:02}:{:02}", (i / 6) % 24, i % 60),
            // Keep the scrolling transcript uniform in height; attachments are
            // demonstrated in the showcase strip above instead.
            attachment: None,
        })
        .collect()
}

/// A batch of older history, labelled relative to the current tail.
fn seed_older(count: usize, offset: usize) -> Vec<Line> {
    (0..count)
        .rev()
        .map(|i| Line {
            author: Author::Bot,
            text: format!("(older) message #{}", offset.saturating_sub(count - i)),
            footer: "earlier".to_owned(),
            attachment: None,
        })
        .collect()
}
