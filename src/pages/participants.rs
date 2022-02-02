//! Poll participants wizard page.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use std::collections::HashSet;

use crate::{
    components::Secrets,
    js::{ExportedData, ExportedDataType},
    layout::{view_data_row, view_err, Card, Icon, RemovalMessage},
    pages::{AppProperties, PageMetadata, PollStageProperties, Route},
    poll::{
        Participant, ParticipantApplication, PollId, PollManager, PollStage, PollState, PublicKey,
        PublicKeyBytes, SecretManagerStatus,
    },
    utils::{value_from_event, Encode, ValidatedValue},
};

#[derive(Debug)]
pub enum ParticipantsMessage {
    ApplicationSet(String),
    Removal(RemovalMessage<PublicKeyBytes>),
    UsAdded,
    ExportRequested(usize),
    SecretUpdated,
    Done,
}

impl ParticipantsMessage {
    fn application_set(event: &Event) -> Self {
        Self::ApplicationSet(value_from_event(event))
    }
}

impl From<RemovalMessage<PublicKeyBytes>> for ParticipantsMessage {
    fn from(message: RemovalMessage<PublicKeyBytes>) -> Self {
        Self::Removal(message)
    }
}

#[derive(Debug)]
pub struct Participants {
    metadata: PageMetadata,
    poll_manager: PollManager,
    poll_id: PollId,
    poll_state: Option<PollState>,
    is_readonly: bool,
    new_application: ValidatedValue,
    validated_application: Option<ParticipantApplication>,
    pending_removals: HashSet<PublicKeyBytes>,
}

impl Participants {
    fn we_are_participant(&self, state: &PollState, ctx: &Context<Self>) -> bool {
        let secrets = AppProperties::from_ctx(ctx).secrets;
        let pk = match secrets.public_key_for_poll(&self.poll_id) {
            Some(pk) => pk,
            None => return false,
        };
        state.has_participant(&pk)
    }

    fn add_participant(&mut self, participant: ParticipantApplication) {
        if let Some(state) = &mut self.poll_state {
            state.insert_participant(participant);
            self.poll_manager.update_poll(&self.poll_id, state);
        }
    }

    fn remove_participant(&mut self, key_bytes: &PublicKeyBytes) {
        if let Some(state) = &mut self.poll_state {
            let idx = state
                .participants()
                .iter()
                .position(|p| p.public_key().as_bytes() == key_bytes);
            if let Some(idx) = idx {
                state.remove_participant(idx);
                self.poll_manager.update_poll(&self.poll_id, state);
            }
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
        self.add_participant(parsed_application);
        self.new_application = ValidatedValue::default();
    }

    fn create_our_participant(&self, ctx: &Context<Self>) -> ParticipantApplication {
        let our_keypair = AppProperties::from_ctx(ctx)
            .secrets
            .keys_for_poll(&self.poll_id)
            .expect_throw("creating participant application with locked secret manager");
        ParticipantApplication::new(&our_keypair, &self.poll_id)
    }

    fn view_poll(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let onexport = AppProperties::from_ctx(ctx).onexport;
        html! {
            <>
                <p class="lead">{ "After poll is created, poll participants must be specified." }</p>
                <p>{ "The participants can submit votes (potentially multiple times, but only \
                the last one will be counted). The participants are also talliers, \
                and must submit tallying shares once the voting stage is finished." }</p>
                <p>
                    <Link<Route> to={Route::Implementation}>
                        { "More about participants →" }
                    </Link<Route>>
                </p>

                { state.spec().view_summary_card(onexport) }

                <h4>{ "Participants" }</h4>
                { self.view_add_us_form(state, ctx) }
                { self.view_participants(state, ctx) }
                { Self::view_shared_key(state) }
            </>
        }
    }

    fn view_participants(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let our_key = AppProperties::from_ctx(ctx)
            .secrets
            .public_key_for_poll(&self.poll_id);

        let participants: Html = state
            .participants()
            .iter()
            .enumerate()
            .map(|(idx, participant)| {
                let card = self.view_participant(idx, participant, our_key.as_ref(), ctx);
                html! { <div class="col-lg-6">{ card }</div> }
            })
            .collect();

        html! {
            <div class="row g-2 mb-3">
                { participants }
                { if self.is_readonly {
                    html!{}
                } else {
                    html!{ <div class="col-lg-6">{ self.view_new_participant_form(ctx) }</div> }
                }}
            </div>
        }
    }

    fn view_participant(
        &self,
        idx: usize,
        participant: &Participant,
        our_key: Option<&PublicKey>,
        ctx: &Context<Self>,
    ) -> Html {
        let is_pending_removal = self
            .pending_removals
            .contains(participant.public_key().as_bytes());
        let title = format!("#{}", idx + 1);
        let mut card = Card::new(
            html! { title },
            html! {
                <p class="card-text mb-0 text-truncate">
                    <strong>{ "Public key:" }</strong>
                    { " " }
                    { participant.public_key().encode() }
                </p>
            },
        );

        let link = ctx.link();
        let key_bytes = participant.public_key_bytes();
        if is_pending_removal {
            card = card.confirm_removal(key_bytes, link);
        }
        if our_key == Some(participant.public_key()) {
            card = card.with_our_mark();
        }
        card = card.with_timestamp(participant.created_at);

        if !is_pending_removal {
            card = card.with_button(html! {
                <button
                    type="button"
                    class="btn btn-sm btn-secondary me-2"
                    title="Copy participant application to clipboard"
                    onclick={link.callback(move |_| {
                        ParticipantsMessage::ExportRequested(idx)
                    })}>
                    { Icon::Export.view() }{ " Export" }
                </button>
            });

            if !self.is_readonly {
                card = card.with_button(html! {
                    <button
                        type="button"
                        class="btn btn-sm btn-danger"
                        title="Remove this participant"
                        onclick={link.callback(move |_| RemovalMessage::Requested(key_bytes))}>
                        { Icon::Remove.view() }{ " Remove" }
                    </button>
                });
            }
        }

        card.view()
    }

    fn view_add_us_form(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let secrets = AppProperties::from_ctx(ctx).secrets;
        let link = ctx.link();

        if secrets.status() == Some(SecretManagerStatus::Unlocked) {
            if self.is_readonly || self.we_are_participant(state, ctx) {
                html! {}
            } else {
                html! {
                    <div class="alert alert-warning py-2" role="alert">
                        { "You are not a vote participant. " }
                        <button
                            type="button"
                            class="btn btn-sm btn-primary align-baseline ms-2"
                            onclick={link.callback(|_| ParticipantsMessage::UsAdded)}>
                            { "Add yourself" }
                        </button>
                    </div>
                }
            }
        } else {
            html! {
                <>
                    { Secrets::view_alert(&secrets, "participant application") }
                    <Secrets ondone={link.callback(|()| ParticipantsMessage::SecretUpdated)} />
                </>
            }
        }
    }

    fn view_new_participant_form(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_application.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        let card = Card::new(
            html! {
                <label for="participant-application">{ "New participant" }</label>
            },
            html! {
                <form>
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
                </form>
            },
        );
        card.with_dotted_border().view()
    }

    fn view_shared_key(state: &PollState) -> Html {
        html! {
            { if let Some(shared_key) = state.shared_key() {
                view_data_row(
                    html! {
                        <label for="shared-key"><strong>{ "Shared key" }</strong></label>
                    },
                    html! {
                        <>
                            <p id="shared-key" class="mb-1 text-truncate">
                                { shared_key.encode() }
                            </p>
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
        }
    }
}

impl Component for Participants {
    type Message = ParticipantsMessage;
    type Properties = PollStageProperties;

    fn create(ctx: &Context<Self>) -> Self {
        let poll_manager = PollManager::default();
        let poll_state = poll_manager.poll(&ctx.props().id);
        let is_readonly = poll_state.as_ref().map_or(true, |state| {
            !matches!(state.stage(), PollStage::Participants { .. })
        });

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
            is_readonly,
            new_application: ValidatedValue::default(),
            validated_application: None,
            pending_removals: HashSet::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ParticipantsMessage::ApplicationSet(application) => {
                self.set_application(application);
            }

            ParticipantsMessage::Removal(RemovalMessage::Requested(key_bytes)) => {
                self.pending_removals.insert(key_bytes);
            }
            ParticipantsMessage::Removal(RemovalMessage::Cancelled(key_bytes)) => {
                self.pending_removals.remove(&key_bytes);
            }
            ParticipantsMessage::Removal(RemovalMessage::Confirmed(key_bytes)) => {
                self.remove_participant(&key_bytes);
                self.pending_removals.remove(&key_bytes);
            }

            ParticipantsMessage::UsAdded => {
                let us = self.create_our_participant(ctx);
                self.add_participant(us);
            }
            ParticipantsMessage::ExportRequested(idx) => {
                if let Some(state) = &self.poll_state {
                    let app = &state.participants()[idx].application;
                    let app = serde_json::to_string_pretty(app)
                        .expect_throw("failed serializing `ParticipantApplication`");
                    AppProperties::from_ctx(ctx).onexport.emit(ExportedData {
                        ty: ExportedDataType::Application,
                        data: app,
                    });
                }
                return false;
            }

            ParticipantsMessage::SecretUpdated => {
                // Do nothing specific, just re-render the component.
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
                    { state.stage().view_nav(PollStage::PARTICIPANTS_IDX, self.poll_id) }
                    { self.view_poll(state, ctx) }

                    { if self.is_readonly {
                        html!{}
                    } else {
                        html! {
                            <div class="mt-4 text-center">
                                <button
                                    type="button"
                                    class="btn btn-primary"
                                    disabled={state.participants().is_empty()}
                                    onclick={link.callback(|_| ParticipantsMessage::Done)}>
                                    { Icon::Check.view() }{ " Next: voting" }
                                </button>
                            </div>
                        }
                    }}
                </>
            }
        } else {
            let history = ctx.link().history().unwrap_throw();
            history.replace(Route::NotFound);
            html! {}
        }
    }
}
