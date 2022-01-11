//! Poll participants wizard page.

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlTextAreaElement};
use yew::{classes, html, Callback, Component, Context, Html, Properties};
use yew_router::prelude::*;

use crate::{
    components::{
        common::{view_data_row, view_err, Icon, ValidatedValue},
        Route,
    },
    poll::{PollId, PollManager, PollParticipant, PollState, SecretManager},
    utils::Encode,
};

#[derive(Debug)]
pub enum ParticipantsMessage {
    ApplicationSet(String),
    ParticipantRemoved(usize),
    ParticipantAdded,
    UsAdded,
    ExportRequested(usize),
}

impl ParticipantsMessage {
    fn application_set(event: &Event) -> Self {
        let target = event.target().expect_throw("no target for change event");
        let target = target
            .dyn_into::<HtmlTextAreaElement>()
            .expect_throw("unexpected target for token set event");
        Self::ApplicationSet(target.value())
    }
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct ParticipantsProperties {
    pub id: PollId,
    #[prop_or_default]
    pub onexport: Callback<String>,
}

#[derive(Debug)]
pub struct Participants {
    secret_manager: SecretManager,
    poll_manager: PollManager,
    poll_id: PollId,
    poll_state: Option<PollState>,
    new_participant: ValidatedValue,
    validated_participant: Option<PollParticipant>,
}

impl Participants {
    fn we_are_participant(&self, state: &PollState) -> bool {
        let pk = self.secret_manager.public_key_for_poll(&self.poll_id);
        state
            .participants
            .iter()
            .any(|participant| participant.public_key == pk)
    }

    fn add_participant(&mut self, participant: PollParticipant) {
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
        self.validated_participant = None;

        let participant = match serde_json::from_str::<PollParticipant>(&application) {
            Ok(participant) => participant,
            Err(err) => {
                self.new_participant = ValidatedValue {
                    value: application,
                    error_message: Some(format!("Error parsing application: {}", err)),
                };
                return;
            }
        };

        self.new_participant = ValidatedValue::unvalidated(application);
        if let Err(err) = participant.validate(&self.poll_id) {
            self.new_participant.error_message =
                Some(format!("Error validating application: {}", err));
            return;
        }
        self.validated_participant = Some(participant);
    }

    fn create_our_participant(&self) -> PollParticipant {
        let our_keypair = self.secret_manager.keys_for_poll(&self.poll_id);
        PollParticipant::new(&our_keypair, &self.poll_id)
    }

    fn view_poll(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <p class="lead">{ "As a second step, poll participants must be specified." }</p>
                <p>{ "Participants will act as poll talliers as well. While voting is not \
                    mandatory, tallying is." }</p>
                <h4>{ "Poll summary "}</h4>
                { state.spec.view_summary() }
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

    fn view_participant(
        &self,
        idx: usize,
        participant: &PollParticipant,
        ctx: &Context<Self>,
    ) -> Html {
        let link = ctx.link();
        let our_key = self.secret_manager.public_key_for_poll(&self.poll_id);
        let our_mark = if participant.public_key == our_key {
            html! { <span class="badge bg-primary ms-2">{ "You" }</span> }
        } else {
            html! {}
        };

        html! {
            <div class="card mb-2">
                <div class="card-body">
                    <h5 class="card-title d-flex">
                        <span class="me-auto">{ "#" }{ &(idx + 1).to_string() }{ our_mark }</span>
                        <div class="btn-group btn-group-sm" role="group" aria-label="Actions">
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
                    </h5>
                    <p class="card-text mb-0">
                        <strong>{ "Public key:" }</strong>
                        { " " }
                        { &participant.public_key.encode() }
                    </p>
                    <p class="card-text">
                        // FIXME: use real date
                        <small class="text-muted">{ "Added on 2022-01-07 13:00:17 UTC" }</small>
                    </p>
                </div>
            </div>
        }
    }

    fn view_actions(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div class="mt-3">
                { if self.we_are_participant(state) {
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
                    disabled={self.validated_participant.is_none()}
                    onclick={link.callback(|_| ParticipantsMessage::ParticipantAdded)}>
                    { Icon::Plus.view() }
                    { " Add participant" }
                </button>
            </div>
        }
    }

    fn view_new_participant_form(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_participant.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        html! {
            <form>
                { if let Some(shared_key) = state.shared_public_key() {
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
                                id="poll-spec"
                                class={control_classes}
                                placeholder="JSON-encoded participant application"
                                value={self.new_participant.value.clone()}
                                onchange={link.callback(|evt| {
                                    ParticipantsMessage::application_set(&evt)
                                })}>
                            </textarea>
                            { if let Some(err) = &self.new_participant.error_message {
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

    fn create(ctx: &Context<Self>) -> Self {
        let poll_manager = PollManager::default();
        let poll_state = poll_manager.poll(&ctx.props().id);
        Self {
            secret_manager: SecretManager::default(),
            poll_manager: PollManager::default(),
            poll_id: ctx.props().id,
            poll_state,
            new_participant: ValidatedValue::default(),
            validated_participant: None,
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
                if let Some(participant) = self.validated_participant.take() {
                    self.add_participant(participant);
                    self.new_participant = ValidatedValue::default();
                }
            }
            ParticipantsMessage::UsAdded => {
                let us = self.create_our_participant();
                self.add_participant(us);
            }
            ParticipantsMessage::ExportRequested(idx) => {
                if let Some(state) = &self.poll_state {
                    let app = serde_json::to_string(&state.participants[idx])
                        .expect_throw("failed serializing `PollParticipant`");
                    ctx.props().onexport.emit(app);
                }
                return false;
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(state) = &self.poll_state {
            html! {
                <>
                    { self.view_poll(state, ctx) }
                    { self.view_new_participant_form(state, ctx) }
                </>
            }
        } else {
            let history = ctx.link().history().unwrap_throw();
            history.replace(Route::NotFound);
            html! {}
        }
    }
}
