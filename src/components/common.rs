//! Common components.

use js_sys::Date;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Callback, Html};

use crate::poll::{PollSpec, PollType, VoteChoice};

pub fn view_local_timestamp(timestamp: f64) -> Html {
    let date = Date::new(&timestamp.into());
    html! {
        <span title="This is a local timestamp; it is not synced among participants">
            { date.to_utc_string() }
        </span>
    }
}

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

type OptionChangeCallback = Callback<(usize, Event)>;

impl PollSpec {
    pub(super) fn view_summary(&self) -> Html {
        self.view(None, None)
    }

    pub(super) fn view_as_form(&self, choice: &VoteChoice, onchange: OptionChangeCallback) -> Html {
        self.view(Some(choice), Some(onchange))
    }

    fn view(&self, choice: Option<&VoteChoice>, onchange: Option<OptionChangeCallback>) -> Html {
        let ty = self.poll_type;
        let options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| {
                let is_selected = choice.map(|choice| choice.is_selected(idx));
                Self::view_option(idx, option, ty, is_selected, onchange.clone())
            })
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

    fn view_option(
        idx: usize,
        option: &str,
        ty: PollType,
        is_selected: Option<bool>,
        onchange: Option<OptionChangeCallback>,
    ) -> Html {
        let control_id = format!("poll-option{}", idx);
        let (control_type, control_name) = match ty {
            PollType::SingleChoice => ("radio", "poll-options".to_owned()),
            PollType::MultiChoice => ("checkbox", control_id.clone()),
        };
        let is_disabled = is_selected.is_none();
        let is_checked = is_selected.unwrap_or(false);
        let onchange = onchange.map(|callback| callback.reform(move |evt| (idx, evt)));

        html! {
            <div class="form-check">
                <input
                    class="form-check-input"
                    type={control_type}
                    name={control_name}
                    id={control_id.clone()}
                    value={idx.to_string()}
                    checked={is_checked}
                    disabled={is_disabled}
                    onchange={onchange} />
                <label class="form-check-label" for={control_id}>{ option }</label>
            </div>
        }
    }
}

/// Component responsible for rendering page metadata via a portal.
#[derive(Debug)]
pub struct PageMetadata {
    pub title: String,
    pub description: String,
    pub is_root: bool,
}

impl PageMetadata {
    pub fn view(&self) -> Html {
        let window = web_sys::window().expect_throw("no Window");
        let document = window.document().expect_throw("no Document");
        let head = document.head().expect_throw("no <head> in Document");
        yew::create_portal(self.view_meta(), head.into())
    }

    fn view_meta(&self) -> Html {
        html! {
            <>
                <meta name="description" content={self.description.clone()} />
                <meta name="og:title" content={self.title.clone()} />
                <meta name="og:description" content={self.description.clone()} />
                <script type="application/ld+json">{ self.linked_data() }</script>
                <title>{ &self.title }{ " | Voting" }</title>
            </>
        }
    }

    fn linked_data(&self) -> String {
        format!(
            r#"{{
                "@context": "https://schema.org/",
                "@type": "{ty}",
                "author": {{
                  "@type": "Person",
                  "name": "Alex Ostrovski"
                }},
                "headline": "{title}",
                "description": "{description}"
            }}"#,
            ty = if self.is_root { "WebSite" } else { "WebPage" },
            title = self.title,
            description = self.description
        )
    }
}
