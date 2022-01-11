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
    pub(super) fn view_summary(&self) -> Html {
        let ty = self.poll_type;
        let options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| Self::view_option_in_summary(idx, option, ty))
            .collect::<Html>();
        html! {
            <>
                <h5>{ &self.title }</h5>
                {if self.description.trim().is_empty() {
                    html! { }
                } else {
                    html! { <p>{ &self.description }</p> }
                }}
                <div>{ options }</div>
            </>
        }
    }

    fn view_option_in_summary(idx: usize, option: &str, ty: PollType) -> Html {
        let control_id = format!("poll-option{}", idx);
        let (control_type, control_name) = match ty {
            PollType::SingleChoice => ("radio", "poll-options".to_owned()),
            PollType::MultiChoice => ("checkbox", control_id.clone()),
        };
        html! {
            <div class="form-check form-check-inline">
                <input
                    class="form-check-input"
                    type={control_type}
                    name={control_name}
                    id={control_id.clone()}
                    value={idx.to_string()}
                    disabled=true />
                <label class="form-check-label" for={control_id}>{ option }</label>
            </div>
        }
    }
}
