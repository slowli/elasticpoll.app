//! New poll wizard page.

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};
use yew::{classes, html, Callback, Component, Context, Html, Properties};

use super::common::{view_data_row, view_err, Icon, ValidatedValue};
use crate::poll::{PollSpec, PollType, MAX_OPTIONS};

#[derive(Debug)]
pub enum NewPollMessage {
    TitleSet(String),
    DescriptionSet(String),
    TypeSet(PollType),
    OptionSet(usize, String),
    OptionRemoved(usize),
    OptionMoved { old_idx: usize, new_idx: usize },
    OptionAdded,
    SpecSet(String),
    SpecReset,
    ExportRequested,
    Done,
}

impl NewPollMessage {
    fn title_set(event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlInputElement>()
            .expect_throw("unexpected target for token set event");
        Self::TitleSet(target.value())
    }

    fn description_set(event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlTextAreaElement>()
            .expect_throw("unexpected target for token set event");
        Self::DescriptionSet(target.value())
    }

    fn option_set(idx: usize, event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlInputElement>()
            .expect_throw("unexpected target for token set event");
        Self::OptionSet(idx, target.value())
    }

    fn type_set(event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlInputElement>()
            .expect_throw("unexpected target for token set event");
        Self::TypeSet(target.value().parse().expect("invalid value"))
    }

    fn spec_set(event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlTextAreaElement>()
            .expect_throw("unexpected target for token set event");
        Self::SpecSet(target.value())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Properties)]
pub struct NewPollProperties {
    #[prop_or_default]
    pub onexport: Callback<String>,
    #[prop_or_default]
    pub ondone: Callback<PollSpec>,
}

/// "New poll" page.
#[derive(Debug)]
pub struct NewPoll {
    title: ValidatedValue,
    description: ValidatedValue,
    poll_type: PollType,
    poll_options: Vec<ValidatedValue>,
    nonce: u64,
    // The `value` is `Some(_)` if there is a problem with parsing it; otherwise, the "Raw" tab
    // renders the JSON presentation of the config.
    spec: ValidatedValue<Option<String>>,
}

impl NewPoll {
    const MAX_FIELD_LEN: usize = 128;
    const MAX_DESCRIPTION_LEN: usize = 1_024;

    fn view_title(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "mb-1"];
        if self.title.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        view_data_row(
            html! {
                <label for="title">{ "Title" }</label>
            },
            html! {
                <>
                    <input
                        type="text"
                        id="title"
                        class={control_classes}
                        maxlength={Self::MAX_FIELD_LEN.to_string()}
                        placeholder="Descriptive poll title"
                        value={self.title.value.clone()}
                        onchange={link.callback(|evt| NewPollMessage::title_set(&evt))} />

                    { if let Some(err) = &self.title.error_message {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </>
            },
        )
    }

    fn view_description(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "mb-1",];
        if self.description.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        view_data_row(
            html! {
                <label for="description">{ "Description" }</label>
            },
            html! {
                <>
                    <textarea
                        id="description"
                        class={control_classes}
                        placeholder="Poll description"
                        maxlength={Self::MAX_DESCRIPTION_LEN.to_string()}
                        onchange={link.callback(|evt| NewPollMessage::description_set(&evt))}>
                        { &self.desription.value }
                    </textarea>

                    { if let Some(err) = &self.description.error_message {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </>
            },
        )
    }

    fn view_poll_type(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        view_data_row(
            html! { <label for="poll-type">{ "Poll type" }</label> },
            html! {
                <>
                    <div class="form-check">
                        <input
                            class="form-check-input"
                            type="radio"
                            name="poll-type"
                            value="single_choice"
                            id="poll-type-single-choice"
                            onchange={link.callback(|evt| NewPollMessage::type_set(&evt))}
                            checked={self.poll_type == PollType::SingleChoice} />
                        <label class="form-check-label" for="poll-type-single-choice">
                            { "Single choice" }
                        </label>
                    </div>
                    <div class="form-check">
                        <input
                            class="form-check-input"
                            type="radio"
                            name="poll-type"
                            value="multi_choice"
                            id="poll-type-multi-choice"
                            onchange={link.callback(|evt| NewPollMessage::type_set(&evt))}
                            checked={self.poll_type == PollType::MultiChoice} />
                        <label class="form-check-label" for="poll-type-multi-choice">
                            { "Multiple choice" }
                        </label>
                    </div>
                </>
            },
        )
    }

    fn view_poll_options(&self, ctx: &Context<Self>) -> Html {
        self.poll_options
            .iter()
            .enumerate()
            .map(|(idx, option)| self.view_poll_option(idx, option, ctx))
            .collect()
    }

    fn view_poll_option(&self, idx: usize, option: &ValidatedValue, ctx: &Context<Self>) -> Html {
        let control_id = format!("option-{}", idx);
        let mut control_classes = classes!["form-control"];
        if option.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        html! {
            <div class="row mb-2">
                <div class="input-group">
                    <input
                        type="text"
                        id={control_id}
                        class={control_classes}
                        placeholder="Option description"
                        value={option.value.clone()}
                        maxlength={Self::MAX_FIELD_LEN.to_string()}
                        onchange={link.callback(move |evt| NewPollMessage::option_set(idx, &evt))}/>
                    { if self.poll_options.len() > 1 {
                        self.view_option_actions(idx, ctx)
                    } else {
                        html!{}
                    }}
                </div>
                { if let Some(err) = &option.error_message {
                    // Add a dummy `span.is-invalid` to show the feedback.
                    html! {
                        <>
                            <span class="is-invalid" />
                            { view_err(err) }
                        </>
                    }
                } else {
                    html!{}
                }}
            </div>
        }
    }

    fn view_option_actions(&self, idx: usize, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <>
                <button
                    type="button"
                    class="btn btn-secondary"
                    title="Move this option upper"
                    disabled={idx == 0}
                    onclick={link.callback(move |_| NewPollMessage::OptionMoved {
                        old_idx: idx,
                        new_idx: idx.saturating_sub(1),
                    })}>
                    { Icon::Up.view() }
                </button>
                <button
                    type="button"
                    class="btn btn-secondary"
                    title="Move this option lower"
                    disabled={idx + 1 == self.poll_options.len()}
                    onclick={link.callback(move |_| NewPollMessage::OptionMoved {
                        old_idx: idx,
                        new_idx: idx + 1,
                    })}>
                    { Icon::Down.view() }
                </button>
                <button
                    type="button"
                    class="btn btn-danger"
                    title="Remove this option"
                    onclick={link.callback(move |_| NewPollMessage::OptionRemoved(idx))}>
                    { Icon::Remove.view() }
                </button>
            </>
        }
    }

    fn validate_title(title: &str) -> Option<String> {
        if title.is_empty() {
            Some("Title cannot be empty".to_owned())
        } else if title.len() > Self::MAX_FIELD_LEN {
            Some(format!(
                "Title length cannot exceed {} bytes",
                Self::MAX_FIELD_LEN
            ))
        } else {
            None
        }
    }

    fn validate_description(description: &str) -> Option<String> {
        if description.len() > Self::MAX_DESCRIPTION_LEN {
            Some(format!(
                "Description length cannot exceed {} bytes",
                Self::MAX_DESCRIPTION_LEN
            ))
        } else {
            None
        }
    }

    fn validate_option(new_option: &str) -> Option<String> {
        if new_option.is_empty() {
            Some("Option title cannot be empty".to_owned())
        } else if new_option.len() > Self::MAX_FIELD_LEN {
            Some(format!(
                "Option title length cannot exceed {} bytes",
                Self::MAX_FIELD_LEN
            ))
        } else {
            // Option uniqueness is validated separately.
            None
        }
    }

    #[allow(clippy::needless_collect)] // false positive
    fn revalidate_options(&mut self) {
        const NON_UNIQUE_MSG: &str = "Option descriptions must be unique";

        let non_unique: Vec<_> = self
            .poll_options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                self.poll_options
                    .iter()
                    .enumerate()
                    .any(|(j, other_option)| i != j && option.value == other_option.value)
            })
            .collect();

        for (non_unique, option) in non_unique.into_iter().zip(&mut self.poll_options) {
            if non_unique && option.error_message.is_none() {
                option.error_message = Some(NON_UNIQUE_MSG.to_owned());
            } else if !non_unique && option.error_message.as_deref() == Some(NON_UNIQUE_MSG) {
                option.error_message = None;
            }
        }
    }

    fn is_valid(&self) -> bool {
        let fields = [
            &self.title.error_message,
            &self.description.error_message,
            &self.spec.error_message,
        ];
        fields
            .into_iter()
            .chain(self.poll_options.iter().map(|option| &option.error_message))
            .all(Option::is_none)
    }

    fn view_tabs_nav() -> Html {
        html! {
            <nav class="nav nav-tabs mb-3">
                <button
                    class="nav-link active"
                    id="edit-poll-tab"
                    data-bs-toggle="tab"
                    data-bs-target="#edit-poll"
                    type="button"
                    role="tab"
                    aria-controls="home"
                    aria-selected="true">
                    <span class="text-muted">{ Icon::Edit.view() }</span>
                    { " Edit" }
                </button>
                <button
                    class="nav-link"
                    id="raw-poll-tab"
                    data-bs-toggle="tab"
                    data-bs-target="#raw-poll"
                    type="button"
                    role="tab"
                    aria-controls="home"
                    aria-selected="false">
                    <span class="text-muted">{ Icon::Import.view() }</span>
                    { " Import / export" }
                </button>
            </nav>
        }
    }

    fn view_tabs(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                { Self::view_tabs_nav() }
                <div class="tab-content">
                    <div
                        class="tab-pane fade show active"
                        id="edit-poll"
                        role="tabpanel"
                        aria-labelledby="edit-poll-tab">

                        { self.view_poll_editor(ctx) }
                    </div>
                    <div
                        class="tab-pane fade"
                        id="raw-poll"
                        role="tabpanel"
                        aria-labelledby="raw-poll-tab">

                        { self.view_raw_poll(ctx) }
                    </div>
                </div>
            </>
        }
    }

    fn view_poll_editor(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <form>
                <div class="mb-3">
                    { self.view_title(ctx) }
                    { self.view_description(ctx) }
                    { self.view_poll_type(ctx) }
                </div>
                <h4>{ "Polling options" }</h4>
                { self.view_poll_options(ctx) }
                { if self.poll_options.len() < MAX_OPTIONS {
                    html! {
                        <button
                            type="button"
                            class="btn btn-outline-secondary"
                            onclick={link.callback(|_| NewPollMessage::OptionAdded)}>
                            { Icon::Plus.view() }
                            { " Add option" }
                        </button>
                    }
                } else {
                    html!{}
                }}
            </form>
        }
    }

    fn view_raw_poll(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes![
            "form-control",
            "font-monospace",
            "small",
            "large-height",
            "mb-1"
        ];
        if self.spec.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        let spec = self.spec.value.clone();
        let spec = spec.unwrap_or_else(|| self.spec_string());
        html! {
            <form>
                <div class="mb-2">
                    <textarea
                        id="poll-spec"
                        class={control_classes}
                        placeholder="JSON-encoded poll spec"
                        value={spec}
                        onchange={link.callback(|evt| NewPollMessage::spec_set(&evt))}>
                    </textarea>
                    { if let Some(err) = &self.spec.error_message {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </div>
                <div>
                    <button
                        type="button"
                        class="btn btn-outline-primary"
                        title="Copy poll parameters to clipboard"
                        onclick={link.callback(move |_| NewPollMessage::ExportRequested)}>
                        { Icon::Export.view() }{ " Export" }
                    </button>
                    { if self.spec.value.is_some() {
                        html! {
                            <button
                                type="button"
                                class="btn btn-outline-secondary ms-2"
                                title="Copy spec from visual editor"
                                onclick={link.callback(move |_| NewPollMessage::SpecReset)}>
                                { Icon::Reset.view() }{ " Reset" }
                            </button>
                        }
                    } else {
                        html!{}
                    }}
                </div>
            </form>
        }
    }

    fn spec(&self) -> PollSpec {
        PollSpec {
            title: self.title.value.clone(),
            description: self.description.value.clone(),
            poll_type: self.poll_type,
            nonce: self.nonce,
            options: self
                .poll_options
                .iter()
                .map(|option| option.value.clone())
                .collect(),
        }
    }

    fn spec_string(&self) -> String {
        let spec = self.spec();
        serde_json::to_string_pretty(&spec).expect_throw("error serializing spec")
    }

    fn set_spec(&mut self, spec_string: String) {
        let spec = match serde_json::from_str::<PollSpec>(&spec_string) {
            Ok(spec) => spec,
            Err(err) => {
                self.spec = ValidatedValue {
                    value: Some(spec_string),
                    error_message: Some(format!("Error deserializing spec: {}", err)),
                };
                return;
            }
        };

        self.spec = ValidatedValue::unvalidated(None);
        self.title = ValidatedValue::new(spec.title, Self::validate_title);
        self.description = ValidatedValue::new(spec.description, Self::validate_description);
        self.poll_type = spec.poll_type;
        self.nonce = spec.nonce;
        self.poll_options = spec
            .options
            .into_iter()
            .map(|description| ValidatedValue::new(description, Self::validate_option))
            .collect();
        self.revalidate_options();
    }

    fn reset_spec(&mut self) {
        self.spec = ValidatedValue::unvalidated(None);
    }
}

impl Component for NewPoll {
    type Message = NewPollMessage;
    type Properties = NewPollProperties;

    fn create(_: &Context<Self>) -> Self {
        Self {
            title: ValidatedValue::unvalidated("Sample poll".to_owned()),
            description: ValidatedValue::default(),
            poll_type: PollType::SingleChoice,
            poll_options: vec![ValidatedValue::unvalidated("Option #1".to_owned())],
            nonce: 0, // FIXME: generate randomly
            spec: ValidatedValue::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, message: Self::Message) -> bool {
        match message {
            NewPollMessage::TitleSet(title) => {
                self.title = ValidatedValue::new(title, Self::validate_title);
            }
            NewPollMessage::DescriptionSet(description) => {
                self.description = ValidatedValue::new(description, Self::validate_description);
            }
            NewPollMessage::TypeSet(ty) => {
                self.poll_type = ty;
            }

            NewPollMessage::OptionSet(idx, description) => {
                self.poll_options[idx] = ValidatedValue::new(description, Self::validate_option);
                self.revalidate_options();
            }
            NewPollMessage::OptionRemoved(idx) => {
                self.poll_options.remove(idx);
                self.revalidate_options();
            }
            NewPollMessage::OptionMoved { old_idx, new_idx } => {
                self.poll_options.swap(old_idx, new_idx);
            }
            NewPollMessage::OptionAdded => {
                let new_description = format!("Option #{}", self.poll_options.len() + 1);
                self.poll_options
                    .push(ValidatedValue::new(new_description, Self::validate_option));
                self.revalidate_options();
            }

            NewPollMessage::SpecSet(spec) => {
                self.set_spec(spec);
            }
            NewPollMessage::SpecReset => {
                self.reset_spec();
            }
            NewPollMessage::ExportRequested => {
                ctx.props().onexport.emit(self.spec_string());
                return false;
            }

            NewPollMessage::Done => {
                ctx.props().ondone.emit(self.spec());
                return false; // There will be a redirect; no need to re-render this page.
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let is_invalid = !self.is_valid();

        let link = ctx.link();
        html! {
            <>
                <p class="lead">{ "First, you need to specify the polling parameters." }</p>
                <p>{ "You can visually edit either visually or directly as JSON. Once the poll \
                    specification is ready, you can export it to share via a reliable broadcast \
                    channel, for example via Telegram or Slack." }</p>
                { self.view_tabs(ctx) }
                <div class="mt-4">
                    <button
                        type="button"
                        class="btn btn-primary"
                        disabled={is_invalid}
                        onclick={link.callback(|_| NewPollMessage::Done)}>
                        { Icon::Check.view() }{ " Proceed to participant selection" }
                    </button>
                </div>
            </>
        }
    }
}
