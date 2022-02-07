//! Layout utils.

use js_sys::Date;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{Element, Event};
use yew::{classes, html, html::Scope, Callback, Component, Html, MouseEvent, NodeRef};

use crate::{
    js::{ExportedData, ExportedDataType},
    poll::{PollSpec, PollType, VoteChoice},
};

fn view_local_timestamp(timestamp: f64) -> Html {
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

/// Shared messages for the removal flow (request, then cancellation or confirmation).
#[derive(Debug)]
pub enum RemovalMessage<T> {
    Requested(T),
    Confirmed(T),
    Cancelled(T),
}

#[derive(Debug)]
pub struct Card {
    our_mark: bool,
    dotted_border: bool,
    confirming_removal: bool,
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
            confirming_removal: false,
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

    pub fn confirm_removal<C, T>(mut self, id: T, link: &Scope<C>) -> Self
    where
        C: Component,
        C::Message: From<RemovalMessage<T>>,
        T: 'static + Copy,
    {
        self.confirming_removal = true;
        self.buttons.push(html! {
            <button
                type="button"
                class="btn btn-sm btn-secondary me-2"
                title="Cancel removal"
                onclick={link.callback(move |_| RemovalMessage::Cancelled(id))}>
                { Icon::Reset.view() }{ " Cancel" }
            </button>
        });
        self.buttons.push(html! {
            <button
                type="button"
                class="btn btn-sm btn-danger"
                title="Confirm removal"
                onclick={link.callback(move |_| RemovalMessage::Confirmed(id))}>
                { Icon::Remove.view() }{ " Remove" }
            </button>
        });
        self
    }

    pub fn view(self) -> Html {
        let mut card_classes = classes!["card", "h-100"];
        if self.dotted_border {
            card_classes.push("border-2");
            card_classes.push("border-dotted");
        }
        if self.confirming_removal {
            card_classes.push("border-danger");
        }

        let our_mark = if self.our_mark {
            html! { <span class="badge bg-primary position-absolute ms-2">{ "You" }</span> }
        } else {
            html! {}
        };

        let title = if self.confirming_removal {
            html! {
                <span class="text-danger">{ "Removing: "}{ self.title }</span>
            }
        } else {
            self.title
        };

        html! {
            <div class={card_classes}>
                <div class="card-body">
                    <h5 class="card-title text-truncate">{ title }{ our_mark }</h5>
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
    pub fn view_summary_card(&self, onexport: &Callback<(ExportedData, Element)>) -> Html {
        let exported_data = ExportedData {
            ty: ExportedDataType::PollSpec,
            data: serde_json::to_string_pretty(self).expect_throw("cannot serialize `PollSpec`"),
        };
        let export_button_ref = NodeRef::default();
        let export_button_ref_ = export_button_ref.clone();
        let onexport = onexport.reform(move |evt: MouseEvent| {
            evt.stop_propagation();
            evt.prevent_default();
            let target = export_button_ref_.cast::<Element>().unwrap_throw();
            (exported_data.clone(), target)
        });

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
                            { "Poll parameters" }
                        </button>
                    </h4>
                    <div id="accordion-body-poll-summary"
                        class="accordion-collapse collapse"
                        aria-labelledby="accordion-header-poll-summary"
                        data-bs-parent="#accordion-poll-summary">

                        <div class="accordion-body">
                            <button
                                ref={export_button_ref}
                                type="button"
                                class="btn btn-sm btn-secondary ms-3 mb-2 float-end"
                                onclick={onexport}>
                                { Icon::Export.view() }{ " Export" }
                            </button>
                            { self.view_summary() }
                        </div>
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

    pub fn view_as_form(&self, choice: &VoteChoice, onchange: &OptionChangeCallback) -> Html {
        self.view(Some(choice), Some(onchange))
    }

    fn view(&self, choice: Option<&VoteChoice>, onchange: Option<&OptionChangeCallback>) -> Html {
        let ty = self.poll_type;
        let options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| {
                let is_selected = choice.map(|choice| choice.is_selected(idx));
                Self::view_option(idx, option, ty, is_selected, onchange.cloned())
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
