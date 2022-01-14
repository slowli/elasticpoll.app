//! Common components.

use js_sys::Date;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Callback, Html};
use yew_router::prelude::*;

use super::Route;
use crate::poll::{PollId, PollSpec, PollStage, PollType, VoteChoice};

fn view_local_timestamp(timestamp: f64) -> Html {
    let date = Date::new(&timestamp.into());
    html! {
        <span title="This is a local timestamp; it is not synced among participants">
            { date.to_utc_string() }
        </span>
    }
}

pub(super) fn view_data_row(label: Html, value: Html) -> Html {
    html! {
        <div class="row mb-1">
            <div class="col-md-4 col-lg-3">{ label }</div>
            <div class="col-md-8 col-lg-9">{ value }</div>
        </div>
    }
}

pub(super) fn view_err(message: &str) -> Html {
    html! {
        <p class="invalid-feedback mb-1">{ message }</p>
    }
}

#[derive(Debug)]
pub(super) struct Card {
    our_mark: bool,
    dotted_border: bool,
    title: Html,
    timestamp: Option<f64>,
    body: Html,
    buttons: Vec<Html>,
}

impl Card {
    pub fn new(title: Html, body: Html) -> Self {
        Self {
            our_mark: false,
            dotted_border: false,
            title,
            timestamp: None,
            body,
            buttons: vec![],
        }
    }

    pub fn with_our_mark(mut self) -> Self {
        self.our_mark = true;
        self
    }

    pub fn with_dotted_border(mut self) -> Self {
        self.dotted_border = true;
        self
    }

    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn with_button(mut self, button: Html) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn view(self) -> Html {
        let mut card_classes = classes!["card", "h-100"];
        if self.dotted_border {
            card_classes.push("border-2");
            card_classes.push("border-dotted");
        }
        let our_mark = if self.our_mark {
            html! { <span class="badge bg-primary position-absolute ms-2">{ "You" }</span> }
        } else {
            html! {}
        };

        html! {
            <div class={card_classes}>
                <div class="card-body">
                    <h5 class="card-title text-truncate">{ self.title }{ our_mark }</h5>
                    { if let Some(timestamp) = self.timestamp {
                        html! {
                            <p class="card-subtitle mb-2 small text-muted">
                                { "Created on " }{ view_local_timestamp(timestamp) }
                            </p>
                        }
                    } else {
                        html!{}
                    }}
                    { self.body }
                </div>
                { if self.buttons.is_empty() {
                    html!{}
                } else {
                    html! { <div class="card-footer">{ for self.buttons }</div> }
                }}
            </div>
        }
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
    pub(super) fn view_summary_card(&self) -> Html {
        html! {
            <div class="accordion mb-3" id="accordion-poll-summary">
                <div class="accordion-item">
                    <h4 class="accordion-header" id="accordion-header-poll-summary">
                        <button
                            type="button"
                            class="accordion-button collapsed"
                            data-bs-toggle="collapse"
                            data-bs-target="#accordion-body-poll-summary"
                            aria-expanded="false"
                            aria-controls="accordion-body-poll-summary">
                            { "Poll summary" }
                        </button>
                    </h4>
                    <div id="accordion-body-poll-summary"
                        class="accordion-collapse collapse"
                        aria-labelledby="accordion-header-poll-summary"
                        data-bs-parent="#accordion-poll-summary">

                        <div class="accordion-body">{ self.view_summary() }</div>
                    </div>
                </div>
            </div>
        }
    }

    fn view_summary(&self) -> Html {
        html! {
            <>
                <h5>{ &self.title }</h5>
                { self.view(None, None) }
            </>
        }
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
                {if self.description.trim().is_empty() {
                    html! { }
                } else {
                    html! { <p class="mb-2">{ &self.description }</p> }
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

impl PollStage {
    /// Renders navigation for poll stages with the current stage selected.
    pub(super) fn view_nav(&self, active_idx: usize, id: PollId) -> Html {
        debug_assert!(self.index() >= active_idx);
        html! {
            <ul class="nav mb-3 nav-pills flex-column flex-md-row justify-content-md-center">
                <li class="nav-item">
                    <a class="nav-link">{ "1. Specification" }</a>
                </li>
                { self.view_nav_item(
                    1,
                    active_idx,
                    Route::PollParticipants { id },
                    "2. Participants",
                ) }
                { self.view_nav_item(
                    2,
                    active_idx,
                    Route::Voting { id },
                    "3. Voting",
                ) }
                <li class="nav-item">
                    <a class="nav-link disabled">{ "4. Tallying" }</a>
                </li>
            </ul>
        }
    }

    fn view_nav_item(&self, idx: usize, active_idx: usize, route: Route, name: &str) -> Html {
        html! {
            <li class="nav-item">
                { if self.index() >= idx {
                    let mut link_classes = classes!["nav-link"];
                    if active_idx == idx {
                        link_classes.push("active");
                    }
                    html! {
                        <Link<Route> to={route} classes={link_classes}>{ name }</Link<Route>>
                    }
                } else {
                    html! { <a class="nav-link disabled">{ name }</a> }
                }}
            </li>
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
