//! A wizard (step-by-step) demo showcasing the `Wizard` component.
//!
//! Run with `cargo run --example wizard --features i18n-fluent`.

use iced::widget::{column, row, scrollable, text};
use iced::{Alignment, Element, Length, Task};
use iced_kit::i18n::{FluentTranslator, I18n};
use iced_kit::widgets::form::{field, form, FormLabelLayout};
use iced_kit::widgets::toggle::{checkbox, switch};
use iced_kit::widgets::{
    button, heading, muted_text, text_input, wizard, Heading, WizardLayout, WizardState, WizardStep,
};
use iced_kit::Theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

struct App {
    dark: bool,
    state: WizardState,
    i18n: I18n<FluentTranslator>,
    // Step 1 — Account
    username: String,
    email: String,
    // Step 2 — Profile
    full_name: String,
    bio: String,
    // Step 3 — Preferences
    notifications: bool,
    dark_mode_pref: bool,
    auto_save: bool,
    // Step 4 — Confirm
    finished: bool,
    cancelled: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    StepChanged(usize, usize),
    Finish,
    Cancel,
    UsernameChanged(String),
    EmailChanged(String),
    FullNameChanged(String),
    BioChanged(String),
    ToggleNotifications(bool),
    ToggleDarkModePref(bool),
    ToggleAutoSave(bool),
    SwitchLanguage(&'static str),
    Reset,
}

impl App {
    fn new() -> Self {
        Self {
            dark: false,
            state: WizardState::new(4),
            i18n: I18n::new(
                FluentTranslator::from_str("en", include_str!("../locales/en/main.ftl"))
                    .expect("valid ftl"),
            ),
            username: String::new(),
            email: String::new(),
            full_name: String::new(),
            bio: String::new(),
            notifications: true,
            dark_mode_pref: false,
            auto_save: true,
            finished: false,
            cancelled: false,
        }
    }

    fn title(&self) -> String {
        "Wizard Demo".to_owned()
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
            Message::ToggleTheme => {
                self.dark = !self.dark;
            }
            Message::StepChanged(_old, new) => {
                if new > self.state.current() {
                    self.state.go_next();
                } else {
                    self.state.go_back();
                }
                self.finished = false;
                self.cancelled = false;
            }
            Message::Finish => {
                self.finished = true;
            }
            Message::Cancel => {
                self.cancelled = true;
            }
            Message::UsernameChanged(v) => self.username = v,
            Message::EmailChanged(v) => self.email = v,
            Message::FullNameChanged(v) => self.full_name = v,
            Message::BioChanged(v) => self.bio = v,
            Message::ToggleNotifications(v) => self.notifications = v,
            Message::ToggleDarkModePref(v) => self.dark_mode_pref = v,
            Message::ToggleAutoSave(v) => self.auto_save = v,
            Message::SwitchLanguage(lang) => {
                self.i18n = I18n::new(
                    FluentTranslator::from_str(
                        lang,
                        if lang == "zh-CN" {
                            include_str!("../locales/zh-CN/main.ftl")
                        } else {
                            include_str!("../locales/en/main.ftl")
                        },
                    )
                    .expect("valid ftl"),
                );
            }
            Message::Reset => {
                *self = Self::new();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let content = if self.finished {
            column![
                heading("Wizard Complete", Heading::H2),
                text("Wizard completed successfully!"),
                button("Reset").on_press(Message::Reset),
            ]
            .spacing(16)
            .align_x(Alignment::Center)
            .into()
        } else if self.cancelled {
            column![
                heading("Wizard Cancelled", Heading::H2),
                text("Wizard was cancelled."),
                button("Start Over").on_press(Message::Reset),
            ]
            .spacing(16)
            .align_x(Alignment::Center)
            .into()
        } else {
            self.wizard_view()
        };

        let theme_toggle =
            button(if self.dark { "Light" } else { "Dark" }).on_press(Message::ToggleTheme);

        let lang_toggle = button(if self.i18n.language() == "en" {
            "中文"
        } else {
            "EN"
        })
        .on_press(Message::SwitchLanguage(if self.i18n.language() == "en" {
            "zh-CN"
        } else {
            "en"
        }));

        let controls = row![theme_toggle, lang_toggle]
            .spacing(8)
            .align_y(Alignment::Center);

        let layout = column![controls, content]
            .spacing(16)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill);

        layout.into()
    }

    fn wizard_view(&self) -> Element<'_, Message, Theme> {
        let i18n = &self.i18n;

        let steps = vec![
            WizardStep::new("Account").description("Your login credentials"),
            WizardStep::new("Profile").description("Tell us about yourself"),
            WizardStep::new("Preferences").description("Customize your experience"),
            WizardStep::new("Confirm").description("Review and finish"),
        ];

        let body = |step: usize| -> Element<'_, Message, Theme> {
            match step {
                0 => self.account_step(),
                1 => self.profile_step(),
                2 => self.preferences_step(),
                3 => self.confirm_step(),
                _ => text("Unknown step").into(),
            }
        };

        wizard(&self.state, steps, Message::StepChanged, body)
            .layout(WizardLayout::Top)
            .height(Length::Fill)
            .show_progress(true)
            .validate(|step, _| match step {
                0 => !self.username.trim().is_empty() && self.email.contains('@'),
                1 => !self.full_name.trim().is_empty(),
                _ => true,
            })
            .i18n(i18n)
            .on_finish(Message::Finish)
            .on_cancel(Message::Cancel)
            .into()
    }

    fn account_step(&self) -> Element<'_, Message, Theme> {
        scrollable(
            column![
                heading("Account", Heading::H2),
                muted_text("Enter your username and email address."),
                form()
                    .layout(FormLabelLayout::Vertical)
                    .child(field().label("Username").description("Required").push(
                        text_input("johndoe", &self.username).on_input(Message::UsernameChanged)
                    ),)
                    .child(field().label("Email").description("Required").push(
                        text_input("john@example.com", &self.email).on_input(Message::EmailChanged)
                    ),),
            ]
            .spacing(12),
        )
        .into()
    }

    fn profile_step(&self) -> Element<'_, Message, Theme> {
        scrollable(
            column![
                heading("Profile", Heading::H2),
                muted_text("Tell us a bit about yourself."),
                form()
                    .layout(FormLabelLayout::Vertical)
                    .child(field().label("Full Name").description("Required").push(
                        text_input("John Doe", &self.full_name).on_input(Message::FullNameChanged)
                    ),)
                    .child(field().label("Bio").description("Optional").push(
                        text_input("A short bio...", &self.bio).on_input(Message::BioChanged)
                    ),),
            ]
            .spacing(12),
        )
        .into()
    }

    fn preferences_step(&self) -> Element<'_, Message, Theme> {
        scrollable(
            column![
                heading("Preferences", Heading::H2),
                muted_text("Customize your experience."),
                checkbox(
                    "Enable notifications",
                    self.notifications,
                    Message::ToggleNotifications
                ),
                switch(
                    "Dark mode",
                    self.dark_mode_pref,
                    Message::ToggleDarkModePref
                ),
                checkbox("Auto-save", self.auto_save, Message::ToggleAutoSave),
            ]
            .spacing(12),
        )
        .into()
    }

    fn confirm_step(&self) -> Element<'_, Message, Theme> {
        scrollable(
            column![
                heading("Confirm", Heading::H2),
                muted_text("Review your information before finishing."),
                form()
                    .layout(FormLabelLayout::Vertical)
                    .child(field().label("Username").push(text(&self.username)))
                    .child(field().label("Email").push(text(&self.email)))
                    .child(field().label("Full Name").push(text(&self.full_name)))
                    .child(field().label("Bio").push(text(if self.bio.is_empty() {
                        "—"
                    } else {
                        &self.bio
                    })),)
                    .child(
                        field()
                            .label("Notifications")
                            .push(text(if self.notifications {
                                "Enabled"
                            } else {
                                "Disabled"
                            })),
                    )
                    .child(
                        field()
                            .label("Dark Mode")
                            .push(text(if self.dark_mode_pref {
                                "Enabled"
                            } else {
                                "Disabled"
                            })),
                    )
                    .child(field().label("Auto-save").push(text(if self.auto_save {
                        "Enabled"
                    } else {
                        "Disabled"
                    })),),
            ]
            .spacing(12),
        )
        .into()
    }
}
