//! A wizard (step-by-step) component.
//!
//! A wizard guides the user through a sequence of steps, showing a step
//! indicator, the current step's content, and navigation buttons.

use std::rc::Rc;

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};

use crate::i18n::{FluentTranslator, I18n};
use crate::theme::Theme;
use crate::widgets::{button, stepper, Step, StepLayout};

/// The direction of a wizard transition, for animation purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardDirection {
    /// Moving forward (toward the last step).
    #[default]
    Forward,
    /// Moving backward (toward the first step).
    Backward,
}

/// The state of a wizard, owned by the caller.
///
/// The wizard itself is stateless — it borrows this state and renders
/// whatever step the state says is current. The caller updates the state
/// in response to messages the wizard emits.
///
/// ```
/// # use iced_kit::widgets::WizardState;
/// let mut state = WizardState::new(3);
/// assert_eq!(state.current(), 0);
/// assert!(state.go_next());
/// assert_eq!(state.current(), 1);
/// assert!(!state.is_last());
/// assert!(state.go_next());
/// assert!(state.is_last());
/// assert!(!state.go_next()); // already at the end
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WizardState {
    current_step: usize,
    step_count: usize,
    direction: WizardDirection,
}

impl WizardState {
    /// Creates a new wizard state with the given number of steps.
    ///
    /// The wizard starts at step 0. `step_count` must be at least 1.
    #[must_use]
    pub fn new(step_count: usize) -> Self {
        Self {
            current_step: 0,
            step_count: step_count.max(1),
            direction: WizardDirection::Forward,
        }
    }

    /// The current step index (zero-based).
    #[must_use]
    pub fn current(&self) -> usize {
        self.current_step
    }

    /// The total number of steps.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.step_count
    }

    /// Whether the wizard is on the first step.
    #[must_use]
    pub fn is_first(&self) -> bool {
        self.current_step == 0
    }

    /// Whether the wizard is on the last step.
    #[must_use]
    pub fn is_last(&self) -> bool {
        self.current_step + 1 >= self.step_count
    }

    /// Whether the wizard can go back.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.current_step > 0
    }

    /// Whether the wizard can go forward.
    #[must_use]
    pub fn can_go_next(&self) -> bool {
        self.current_step + 1 < self.step_count
    }

    /// Attempts to advance to the next step.
    ///
    /// Returns `true` if the step was advanced, `false` if already at the end.
    pub fn go_next(&mut self) -> bool {
        if self.can_go_next() {
            self.direction = WizardDirection::Forward;
            self.current_step += 1;
            true
        } else {
            false
        }
    }

    /// Attempts to go back to the previous step.
    ///
    /// Returns `true` if the step was moved back, `false` if already at the start.
    pub fn go_back(&mut self) -> bool {
        if self.can_go_back() {
            self.direction = WizardDirection::Backward;
            self.current_step -= 1;
            true
        } else {
            false
        }
    }

    /// Attempts to jump to a specific step.
    ///
    /// Returns `true` if the step was changed, `false` if the index is out of bounds.
    pub fn go_to(&mut self, step: usize) -> bool {
        if step < self.step_count {
            self.direction = if step > self.current_step {
                WizardDirection::Forward
            } else {
                WizardDirection::Backward
            };
            self.current_step = step;
            true
        } else {
            false
        }
    }

    /// The progress as a value between 0.0 and 1.0.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.step_count <= 1 {
            1.0
        } else {
            self.current_step as f32 / (self.step_count - 1) as f32
        }
    }

    /// The direction of the last transition.
    #[must_use]
    pub fn direction(&self) -> WizardDirection {
        self.direction
    }
}

/// One step of a wizard.
#[derive(Debug, Clone)]
#[must_use = "a WizardStep does nothing unless it is given to `wizard`"]
pub struct WizardStep {
    title: String,
    description: Option<String>,
    icon: Option<crate::icons::IconName>,
}

impl WizardStep {
    /// Creates a step with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
        }
    }

    /// Sets the step's description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the step's icon.
    pub fn icon(mut self, icon: crate::icons::IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The step's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The step's description.
    #[must_use]
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The step's icon.
    #[must_use]
    pub fn get_icon(&self) -> Option<crate::icons::IconName> {
        self.icon
    }
}

/// The layout of a wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardLayout {
    /// Stepper at the top, content below (default).
    #[default]
    Top,
    /// Stepper on the left, content on the right.
    Left,
    /// No stepper, just a progress bar.
    Progress,
}

/// A wizard (step-by-step) component.
///
/// The wizard shows a step indicator, the current step's content, and
/// navigation buttons. It is stateless — the caller owns the [`WizardState`]
/// and passes it in on every render.
///
/// # Example
///
/// ```
/// # use iced_kit::widgets::{wizard, WizardState, WizardStep};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { StepChanged(usize, usize), Finish }
/// # fn view(state: &WizardState) -> Element<'static, Message, Theme> {
/// let steps = vec![
///     WizardStep::new("Account"),
///     WizardStep::new("Profile"),
///     WizardStep::new("Confirm"),
/// ];
///
/// wizard(state, steps, Message::StepChanged, |step| {
///     iced::widget::text(format!("Step {step} content")).into()
/// })
/// .on_finish(Message::Finish)
/// .into()
/// # }
/// ```
#[must_use = "a Wizard does nothing unless it is turned into an Element"]
pub struct Wizard<'a, Message> {
    state: &'a WizardState,
    steps: Vec<WizardStep>,
    body: Box<dyn Fn(usize) -> Element<'a, Message, Theme> + 'a>,
    on_finish: Option<Message>,
    on_cancel: Option<Message>,
    on_step_change: Rc<dyn Fn(usize, usize) -> Message + 'a>,
    layout: WizardLayout,
    show_progress: bool,
    validate: Option<Rc<dyn Fn(usize, &WizardState) -> bool + 'a>>,
    i18n: Option<&'a I18n<FluentTranslator>>,
    height: Length,
}

impl<'a, Message: Clone + 'a> Wizard<'a, Message> {
    /// Creates a wizard.
    ///
    /// `on_step_change` is required — it is called whenever the user navigates
    /// between steps, with `(old_step, new_step)` as arguments.
    pub fn new(
        state: &'a WizardState,
        steps: Vec<WizardStep>,
        on_step_change: impl Fn(usize, usize) -> Message + 'a,
        body: impl Fn(usize) -> Element<'a, Message, Theme> + 'a,
    ) -> Self {
        Self {
            state,
            steps,
            body: Box::new(body),
            on_finish: None,
            on_cancel: None,
            on_step_change: Rc::new(on_step_change),
            layout: WizardLayout::default(),
            show_progress: false,
            validate: None,
            i18n: None,
            height: Length::Shrink,
        }
    }

    /// Sets the wizard's overall height.
    ///
    /// When set to a fixed value or `Length::Fill`, the body area expands to
    /// fill the available space and the navigation buttons stay pinned to the
    /// bottom. The default is `Length::Shrink`, where the wizard takes only
    /// as much height as its content needs.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the layout.
    pub fn layout(mut self, layout: WizardLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Shows a progress bar below the stepper.
    pub fn show_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Sets a validation closure.
    ///
    /// The closure receives `(current_step, state)` and returns `true` if the
    /// user is allowed to leave the current step. When it returns `false`, the
    /// Next and Finish buttons are disabled.
    pub fn validate(mut self, validate: impl Fn(usize, &WizardState) -> bool + 'a) -> Self {
        self.validate = Some(Rc::new(validate));
        self
    }

    /// Sets the i18n context for button labels.
    pub fn i18n(mut self, i18n: &'a I18n<FluentTranslator>) -> Self {
        self.i18n = Some(i18n);
        self
    }

    /// Reports when the wizard finishes (the Finish button is pressed).
    pub fn on_finish(mut self, on_finish: Message) -> Self {
        self.on_finish = Some(on_finish);
        self
    }

    /// Reports when the wizard is cancelled (the Cancel button is pressed).
    pub fn on_cancel(mut self, on_cancel: Message) -> Self {
        self.on_cancel = Some(on_cancel);
        self
    }

    /// Translates a key using the wizard's i18n context, or returns the fallback.
    fn tr(&self, key: &str, fallback: &str) -> String {
        match self.i18n {
            Some(i18n) => {
                let translated = i18n.tr(key);
                if translated == key {
                    fallback.to_owned()
                } else {
                    translated
                }
            }
            None => fallback.to_owned(),
        }
    }

    /// Whether the user can proceed from the current step.
    fn can_proceed(&self) -> bool {
        self.validate
            .as_ref()
            .map_or(true, |v| v(self.state.current_step, self.state))
    }

    /// Turns the wizard into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let can_proceed = self.can_proceed();
        let back_label = self.tr("wizard-back", "Back");
        let next_label = self.tr("wizard-next", "Next");
        let finish_label = self.tr("wizard-finish", "Finish");
        let cancel_label = self.tr("wizard-cancel", "Cancel");

        let Self {
            state,
            steps,
            body,
            on_finish,
            on_cancel,
            on_step_change,
            layout,
            show_progress,
            validate: _,
            i18n: _,
            height,
        } = self;

        let current = state.current_step;
        let is_first = state.is_first();
        let is_last = state.is_last();

        // Build the step indicator
        let step_indicator: Element<'a, Message, Theme> = match layout {
            WizardLayout::Top | WizardLayout::Left => {
                let step_items: Vec<Step> = steps
                    .iter()
                    .map(|s| {
                        let mut step = Step::new(s.title.clone());
                        if let Some(icon) = s.icon {
                            step = step.icon(icon);
                        }
                        step
                    })
                    .collect();

                let on_step_change_clone = on_step_change.clone();
                let mut stepper_widget = stepper(step_items, current)
                    .on_select(move |target| on_step_change_clone(current, target));

                if layout == WizardLayout::Left {
                    stepper_widget = stepper_widget.layout(StepLayout::Vertical);
                }

                stepper_widget.into()
            }
            WizardLayout::Progress => {
                let progress = state.progress();
                iced::widget::progress_bar(0.0..=1.0, progress).into()
            }
        };

        // Build the body content
        let body_content = body(current);

        let mut nav_row = row![].spacing(8).align_y(Alignment::Center);

        // Cancel button
        if let Some(ref on_cancel) = on_cancel {
            nav_row = nav_row.push(button(cancel_label).on_press(on_cancel.clone()));
        }

        nav_row = nav_row.push(Space::new().width(Length::Fill));

        // Back button
        if !is_first {
            let on_step_change_clone = on_step_change.clone();
            nav_row = nav_row
                .push(button(back_label).on_press(on_step_change_clone(current, current - 1)));
        }

        // Next/Finish button
        if is_last {
            if let Some(ref on_finish) = on_finish {
                let mut btn = button(finish_label).primary();
                if !can_proceed {
                    btn = btn.disabled(true);
                }
                nav_row = nav_row.push(btn.on_press(on_finish.clone()));
            }
        } else {
            let on_step_change_clone = on_step_change.clone();
            let mut btn = button(next_label).primary();
            if !can_proceed {
                btn = btn.disabled(true);
            }
            nav_row = nav_row.push(btn.on_press(on_step_change_clone(current, current + 1)));
        }

        // The body fills all remaining space so the nav row sits at the bottom.
        let body_fill = container(body_content).height(Length::Fill);

        let wizard_column = if show_progress && layout != WizardLayout::Progress {
            column![
                step_indicator,
                body_fill,
                iced::widget::progress_bar(0.0..=1.0, state.progress()),
                nav_row,
            ]
            .spacing(16)
        } else {
            column![step_indicator, body_fill, nav_row].spacing(16)
        };

        container(wizard_column).height(height).into()
    }
}

impl<'a, Message: Clone + 'a> From<Wizard<'a, Message>> for Element<'a, Message, Theme> {
    fn from(wizard: Wizard<'a, Message>) -> Self {
        wizard.into_element()
    }
}

/// Builds a wizard.
///
/// This is a shorthand for [`Wizard::new`].
pub fn wizard<'a, Message: Clone + 'a>(
    state: &'a WizardState,
    steps: Vec<WizardStep>,
    on_step_change: impl Fn(usize, usize) -> Message + 'a,
    body: impl Fn(usize) -> Element<'a, Message, Theme> + 'a,
) -> Wizard<'a, Message> {
    Wizard::new(state, steps, on_step_change, body)
}
