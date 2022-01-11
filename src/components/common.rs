//! Common components.

use yew::{classes, html, Html};

use crate::poll::{PollSpec, PollType};

pub fn view_data_row(label: Html, value: Html) -> Html {
    html! {
        <div class="row mb-1">
            <div class="col-md-4 col-lg-3">{ label }</div>
            <div class="col-md-8 col-lg-9">{ value }</div>
        </div>
    }
}

pub fn view_err(message: &str) -> Html {
    html! {
        <p class="invalid-feedback mb-1">{ message }</p>
    }
}

/// Value together with validation errors.
#[derive(Debug, Default)]
pub(super) struct ValidatedValue<T = String> {
    pub value: T,
    pub error_message: Option<String>,
}

impl<T> ValidatedValue<T> {
    pub fn unvalidated(value: T) -> Self {
        Self {
            value,
            error_message: None,
        }
    }
}

impl ValidatedValue {
    pub fn new(value: String, check: impl FnOnce(&str) -> Option<String>) -> Self {
        Self {
            error_message: check(&value),
            value,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Remove,
    Up,
    Down,
    Plus,
    Edit,
    Import,
    Export,
    Reset,
    Check,
}

impl Icon {
    fn icon_class(self) -> &'static str {
        match self {
            Self::Remove => "bi-x-lg",
            Self::Up => "bi-arrow-up",
            Self::Down => "bi-arrow-down",
            Self::Plus => "bi-plus-lg",
            Self::Edit => "bi-pencil",
            Self::Import => "bi-code-slash",
            Self::Export => "bi-clipboard",
            Self::Reset => "bi-backspace",
            Self::Check => "bi-check-lg",
        }
    }

    pub fn view(self) -> Html {
        html! { <i class={classes!("bi", self.icon_class())}></i> }
    }
}

impl PollSpec {
    // TODO: rework to look more intuitive
    pub(super) fn view_summary(&self) -> Html {
        let options = self
            .options
            .iter()
            .map(|option| Self::view_option_in_summary(option))
            .collect::<Html>();
        let poll_type = match self.poll_type {
            PollType::SingleChoice => "Single choice",
            PollType::MultiChoice => "Multiple choice",
        };
        html! {
            <>
                { view_data_row(
                    html! { <label for="poll-title"><strong>{ "Title" }</strong></label> },
                    html! { <div id="poll-title">{ &self.title }</div> },
                ) }
                { view_data_row(
                    html! {
                        <label for="poll-description">
                            <strong>{ "Description" }</strong>
                        </label>
                    },
                    html! {
                        <div id="poll-description">
                            {if self.description.trim().is_empty() {
                                html! { <em>{ "(No description provided)" }</em> }
                            } else {
                                html! { &self.description }
                            }}
                        </div>
                    },
                ) }
                { view_data_row(
                    html! { <label for="poll-type"><strong>{ "Type" }</strong></label> },
                    html! { <div id="poll-type">{ poll_type }</div> },
                ) }
                { view_data_row(
                    html! { <label for="poll-options"><strong>{ "Options" }</strong></label> },
                    html! { <ul id="poll-options" class="list-unstyled">{ options }</ul> },
                ) }
            </>
        }
    }

    fn view_option_in_summary(option: &str) -> Html {
        html! { <li>{ option }</li> }
    }
}
