//! Poll participants wizard page.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Callback, Component, Context, Html, Properties};
use yew_router::prelude::*;

use std::rc::Rc;

use super::{
    common::{view_data_row, view_err, view_local_timestamp, Icon, PageMetadata, ValidatedValue},
    Route,
};
use crate::{
    poll::{Participant, ParticipantApplication, PollId, PollManager, PollState, SecretManager},
    utils::{value_from_event, Encode},
};

#[derive(Debug)]
pub enum ParticipantsMessage {
    ApplicationSet(String),
    ParticipantRemoved(usize),
    ParticipantAdded,
    UsAdded,
    ExportRequested(usize),
    Done,
}

impl ParticipantsMessage {
    fn application_set(event: &Event) -> Self {
        Self::ApplicationSet(value_from_event(event))
    }
}

#[derive(Debug, Clone, Properties)]
pub struct ParticipantsProperties {
    pub id: PollId,
    pub secrets: Rc<SecretManager>,
    #[prop_or_default]
    pub onexport: Callback<String>,
    #[prop_or_default]
    pub ondone: Callback<PollState>,
}

impl PartialEq for ParticipantsProperties {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && Rc::ptr_eq(&self.secrets, &other.secrets)
            && self.onexport == other.onexport
            && self.ondone == other.ondone
    }
}

#[derive(Debug)]
pub struct Participants {
    metadata: PageMetadata,
    poll_manager: PollManager,
    poll_id: PollId,
    poll_state: Option<PollState>,
    new_application: ValidatedValue,
    validated_application: Option<ParticipantApplication>,
}

impl Participants {
    fn we_are_participant(&self, state: &PollState, ctx: &Context<Self>) -> bool {
        let pk = ctx.props().secrets.public_key_for_poll(&self.poll_id);
        state
            .participants
            .iter()
            .any(|participant| *participant.public_key() == pk)
    }

    fn add_participant(&mut self, participant: ParticipantApplication) {
        if let Some(state) = &mut self.poll_state {
            state.insert_participant(participant);
            self.poll_manager.update_poll(&self.poll_id, state);
        }
    }

    fn remove_participant(&mut self, idx: usize) {
        if let Some(state) = &mut self.poll_state {
            state.participants.remove(idx);
            self.poll_manager.update_poll(&self.poll_id, state);
        }
    }

    fn set_application(&mut self, application: String) {
        self.validated_application = None;

        let parsed_application: ParticipantApplication = match serde_json::from_str(&application) {
            Ok(application) => application,
            Err(err) => {
                self.new_application = ValidatedValue {
                    value: application,
                    error_message: Some(format!("Error parsing application: {}", err)),
                };
                return;
            }
        };

        self.new_application = ValidatedValue::unvalidated(application);
        if let Err(err) = parsed_application.validate(&self.poll_id) {
            self.new_application.error_message =
                Some(format!("Error validating application: {}", err));
            return;
        }
        self.validated_application = Some(parsed_application);
    }

    fn create_our_participant(&self, ctx: &Context<Self>) -> ParticipantApplication {
        let our_keypair = ctx.props().secrets.keys_for_poll(&self.poll_id);
        ParticipantApplication::new(&our_keypair, &self.poll_id)
    }

    fn view_poll(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <p class="lead">{ "As a second step, poll participants must be specified." }</p>
                <p>{ "Participants will act as poll talliers as well. While voting is not \
                    mandatory, tallying is." }</p>

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

                            <div class="accordion-body">{ state.spec.view_summary() }</div>
                        </div>
                    </div>
                </div>

                <h4>{ "Participants" }</h4>
                { self.view_participants(state, ctx) }
            </>
        }
    }

    fn view_participants(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let participants: Html = state
            .participants
            .iter()
            .enumerate()
            .map(|(idx, participant)| self.view_participant(idx, participant, ctx))
            .collect();
        html! {
            <div class="mb-3">
                { participants }
                { if state.participants.is_empty() {
                    html! {
                        <div class="text-muted"><em>{ "(No participants yet)" }</em></div>
                    }
                } else {
                    html!{}
                }}
            </div>
        }
    }

    fn view_participant(&self, idx: usize, participant: &Participant, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let our_key = ctx.props().secrets.public_key_for_poll(&self.poll_id);
        let our_mark = if *participant.public_key() == our_key {
            html! { <span class="badge bg-primary ms-2">{ "You" }</span> }
        } else {
            html! {}
        };

        html! {
            <div class="card mb-2">
                <div class="card-body">
                    <div class="btn-group btn-group-sm float-end ms-2 mb-2"
                        role="group"
                        aria-label="Actions">

                        <button
                            type="button"
                            class="btn btn-secondary"
                            title="Copy participant application to clipboard"
                            onclick={link.callback(move |_| {
                                ParticipantsMessage::ExportRequested(idx)
                            })}>
                            { Icon::Export.view() }
                        </button>
                        <button
                            type="button"
                            class="btn btn-danger"
                            title="Remove this participant"
                            onclick={link.callback(move |_| {
                                ParticipantsMessage::ParticipantRemoved(idx)
                            })}>
                            { Icon::Remove.view() }
                        </button>
                    </div>
                    <h5 class="card-title">{ "#" }{ &(idx + 1).to_string() }{ our_mark }</h5>
                    <p class="card-subtitle mb-2 small text-muted">
                        { "Added on " }{ view_local_timestamp(participant.created_at) }
                    </p>
                    <p class="card-text mb-0">
                        <strong>{ "Public key:" }</strong>
                        { " " }
                        { participant.public_key().encode() }
                    </p>
                </div>
            </div>
        }
    }

    fn view_actions(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div class="mt-3">
                { if self.we_are_participant(state, ctx) {
                    html!{}
                } else {
                    html! {
                        <button
                            type="button"
                            class="btn btn-outline-primary me-2"
                            onclick={link.callback(|_| ParticipantsMessage::UsAdded)}>
                            { Icon::Plus.view() }{ " Add yourself" }
                        </button>
                    }
                }}
                <button
                    type="button"
                    class="btn btn-outline-secondary"
                    disabled={self.validated_application.is_none()}
                    onclick={link.callback(|_| ParticipantsMessage::ParticipantAdded)}>
                    { Icon::Plus.view() }
                    { " Add participant" }
                </button>
            </div>
        }
    }

    fn view_new_participant_form(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_application.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        html! {
            <form>
                { if let Some(shared_key) = state.shared_key() {
                    view_data_row(
                        html! {
                            <label for="shared-key"><strong>{ "Shared key" }</strong></label>
                        },
                        html! {
                            <>
                            <p id="shared-key" class="mb-1">{ shared_key.encode() }</p>
                            <p class="small text-muted">
                                { "The order of participants does not matter and can differ for \
                                different participants. However, this shared public key \
                                must be the same across all participants before proceeding \
                                to the next step." }
                            </p>
                            </>
                        },
                    )
                } else {
                    html!{}
                }}
                { view_data_row(
                    html! {
                        <label for="participant-application">
                            <strong>{ "Participant application" }</strong>
                        </label>
                    },
                    html! {
                        <>
                            <textarea
                                id="participant-application"
                                class={control_classes}
                                placeholder="JSON-encoded participant application"
                                value={self.new_application.value.clone()}
                                onchange={link.callback(|evt| {
                                    ParticipantsMessage::application_set(&evt)
                                })}>
                            </textarea>
                            { if let Some(err) = &self.new_application.error_message {
                                view_err(err)
                            } else {
                                html!{}
                            }}
                        </>
                    },
                )}
                { self.view_actions(state, ctx) }
            </form>
        }
    }
}

impl Component for Participants {
    type Message = ParticipantsMessage;
    type Properties = ParticipantsProperties;

    // FIXME: react to poll state
    fn create(ctx: &Context<Self>) -> Self {
        let poll_manager = PollManager::default();
        let poll_state = poll_manager.poll(&ctx.props().id);
        Self {
            metadata: PageMetadata {
                title: "Configure participants for poll".to_owned(),
                description: "Configure cryptographic identities (public keys) of \
                    eligible voters and talliers."
                    .to_owned(),
                is_root: false,
            },
            poll_manager: PollManager::default(),
            poll_id: ctx.props().id,
            poll_state,
            new_application: ValidatedValue::default(),
            validated_application: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ParticipantsMessage::ApplicationSet(application) => {
                self.set_application(application);
            }
            ParticipantsMessage::ParticipantRemoved(idx) => {
                self.remove_participant(idx);
            }
            ParticipantsMessage::ParticipantAdded => {
                if let Some(participant) = self.validated_application.take() {
                    self.add_participant(participant);
                    self.new_application = ValidatedValue::default();
                }
            }
            ParticipantsMessage::UsAdded => {
                let us = self.create_our_participant(ctx);
                self.add_participant(us);
            }
            ParticipantsMessage::ExportRequested(idx) => {
                if let Some(state) = &self.poll_state {
                    let app = &state.participants[idx].application;
                    let app = serde_json::to_string_pretty(app)
                        .expect_throw("failed serializing `ParticipantApplication`");
                    ctx.props().onexport.emit(app);
                }
                return false;
            }

            ParticipantsMessage::Done => {
                let state = self.poll_state.take().expect_throw("no poll state");
                ctx.props().ondone.emit(state);
                return false; // There will be a redirect; no need to re-render this page.
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(state) = &self.poll_state {
            let link = ctx.link();
            html! {
                <>
                    { self.metadata.view() }
                    { self.view_poll(state, ctx) }
                    { self.view_new_participant_form(state, ctx) }
                    <div class="mt-4 text-center">
                        <button
                            type="button"
                            class="btn btn-primary"
                            disabled={state.participants.is_empty()}
                            onclick={link.callback(|_| ParticipantsMessage::Done)}>
                            { Icon::Check.view() }{ " Next: voting" }
                        </button>
                    </div>
                </>
            }
        } else {
            let history = ctx.link().history().unwrap_throw();
            history.replace(Route::NotFound);
            html! {}
        }
    }
}
