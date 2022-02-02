//! Home page.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use std::{cmp::Ordering, collections::HashSet};

use crate::{
    js::{ExportedData, ExportedDataType},
    layout::{view_err, Card, Icon, RemovalMessage},
    pages::{AppProperties, PageMetadata, Route},
    poll::{ExportedPoll, PollId, PollManager, PollStage, PollState},
    utils::{value_from_event, ValidatedValue},
};

#[derive(Debug)]
pub enum HomeMessage {
    PollSet(String),
    ExportRequested(PollId),
    Removal(RemovalMessage<PollId>),
}

impl HomeMessage {
    fn poll_set(event: &Event) -> Self {
        Self::PollSet(value_from_event(event))
    }
}

impl From<RemovalMessage<PollId>> for HomeMessage {
    fn from(message: RemovalMessage<PollId>) -> Self {
        Self::Removal(message)
    }
}

/// Home page component.
#[derive(Debug)]
pub struct Home {
    poll_manager: PollManager,
    metadata: PageMetadata,
    new_poll: ValidatedValue,
    pending_removals: HashSet<PollId>,
}

impl Home {
    fn set_poll(&mut self, poll: String) {
        let parsed_poll = match serde_json::from_str::<ExportedPoll>(&poll) {
            Ok(poll) => poll,
            Err(err) => {
                self.new_poll = ValidatedValue {
                    value: poll,
                    error_message: Some(format!("Error parsing poll: {}", err)),
                };
                return;
            }
        };

        let (poll_id, imported_poll) = match PollState::import(parsed_poll) {
            Ok(value) => value,
            Err(err) => {
                self.new_poll = ValidatedValue {
                    value: poll,
                    error_message: Some(format!("Error validating poll: {}", err)),
                };
                return;
            }
        };
        self.poll_manager.update_poll(&poll_id, &imported_poll);
        self.new_poll = ValidatedValue::default();
    }

    fn view_polls(&self, ctx: &Context<Self>) -> Html {
        let mut polls = self.poll_manager.polls();
        polls.sort_unstable_by(|(_, poll), (_, other_poll)| {
            poll.created_at
                .partial_cmp(&other_poll.created_at)
                .unwrap_or(Ordering::Equal)
        });

        let polls: Html = polls
            .into_iter()
            .map(|(id, state)| {
                html! { <div class="col-lg-6">{ self.view_poll(id, &state, ctx) }</div> }
            })
            .collect();
        html! {
            <>
                <div class="row g-2 mb-2">
                    { polls }
                </div>
                <div class="mb-3">
                    <Link<Route>
                        to={Route::NewPoll}
                        classes={classes!["btn", "btn-outline-primary"]}>
                        {Icon::Plus.view()}{ " Create new poll" }
                    </Link<Route>>
                </div>
                <h5 class="text-muted">{ "Import poll" }</h5>
                { self.view_poll_import_form(ctx) }
            </>
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn view_poll(&self, id: PollId, state: &PollState, ctx: &Context<Self>) -> Html {
        let poll_stage = state.stage();
        let progress_percent = (poll_stage.index() as f64 / PollStage::MAX_INDEX as f64) * 100.0;
        let is_pending_removal = self.pending_removals.contains(&id);

        let link = ctx.link();
        let mut card = Card::new(
            html! { &state.spec().title },
            html! {
                <>
                    <p class="card-text mb-1">{ Self::view_poll_stage(poll_stage) }</p>
                    <div class="progress mb-2" style="height: 2px;">
                        <div
                            class="progress-bar"
                            role="progressbar"
                            style={format!("width: {:.2}%", progress_percent)}
                            aria-valuenow={poll_stage.index().to_string()}
                            aria-valuemin="0"
                            aria-valuemax={PollStage::MAX_INDEX.to_string()}>
                        </div>
                    </div>
                </>
            },
        );
        if is_pending_removal {
            card = card.confirm_removal(id, link);
        }

        let continue_text = if matches!(poll_stage, PollStage::Finished) {
            "Results"
        } else {
            "Continue"
        };
        let mut card = card.with_timestamp(state.created_at);
        if !is_pending_removal {
            card = card
                .with_button(html! {
                    <Link<Route>
                        to={Route::for_poll(id, poll_stage)}
                        classes={classes!["btn", "btn-sm", "btn-primary", "me-2"]}>
                        { continue_text }
                    </Link<Route>>
                })
                .with_button(html! {
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary me-2"
                        title="Copy poll state to clipboard"
                        onclick={link.callback(move |_| HomeMessage::ExportRequested(id))}>
                        { Icon::Export.view() }{ " Export" }
                    </button>
                })
                .with_button(html! {
                    <button
                        type="button"
                        class="btn btn-sm btn-danger"
                        title="Remove this poll"
                        onclick={link.callback(move |_| RemovalMessage::Requested(id))}>
                        { Icon::Remove.view() }{ " Remove" }
                    </button>
                });
        };
        card.view()
    }

    fn view_poll_stage(stage: PollStage) -> Html {
        match stage {
            PollStage::Participants { participants } => {
                html! {
                    <>
                        <strong>{ "Adding participants:" }</strong>
                        { format!(" {}", participants) }
                    </>
                }
            }
            PollStage::Voting {
                votes,
                participants,
            } => {
                html! {
                    <>
                        <strong>{ "Voting:" }</strong>
                        { format!(" {} votes / {} eligible voters", votes, participants) }
                    </>
                }
            }
            PollStage::Tallying {
                shares,
                participants,
            } => {
                html! {
                    <>
                        <strong>{ "Tallying:" }</strong>
                        { format!(" {} shares / {} talliers", shares, participants) }
                    </>
                }
            }
            PollStage::Finished => {
                html! { <strong>{ "Finished" }</strong> }
            }
        }
    }

    fn view_poll_import_form(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_poll.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        html! {
            <form>
                <textarea
                    id="encoded-poll"
                    class={control_classes}
                    placeholder="JSON-encoded poll state"
                    value={self.new_poll.value.clone()}
                    onchange={link.callback(|evt| HomeMessage::poll_set(&evt))}>
                </textarea>
                { if let Some(err) = &self.new_poll.error_message {
                    view_err(err)
                } else {
                    html!{}
                }}
            </form>
        }
    }
}

impl Component for Home {
    type Message = HomeMessage;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            metadata: PageMetadata {
                title: "Welcome".to_owned(),
                description: "A fully contained WASM web app allowing to hold polls \
                    in a cryptographically secure and private manner."
                    .to_owned(),
                is_root: true,
            },
            poll_manager: PollManager::default(),
            new_poll: ValidatedValue::default(),
            pending_removals: HashSet::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            HomeMessage::PollSet(poll) => {
                self.set_poll(poll);
            }

            HomeMessage::Removal(RemovalMessage::Requested(id)) => {
                self.pending_removals.insert(id);
            }
            HomeMessage::Removal(RemovalMessage::Confirmed(id)) => {
                self.poll_manager.remove_poll(&id);
                self.pending_removals.remove(&id);
            }
            HomeMessage::Removal(RemovalMessage::Cancelled(id)) => {
                self.pending_removals.remove(&id);
            }

            HomeMessage::ExportRequested(id) => {
                if let Some(poll) = self.poll_manager.poll(&id) {
                    let data = serde_json::to_string_pretty(&poll.export())
                        .expect_throw("Cannot serialize `ExportedPoll`");
                    AppProperties::from_ctx(ctx).onexport.emit(ExportedData {
                        ty: ExportedDataType::PollState,
                        data,
                    });
                    return false;
                }
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                { self.metadata.view() }
                <p class="lead">{
                    "Elastic poll is a small web app that allows organizing single-choice and \
                     multi-choice polls that combine privacy and universal verifiability with \
                     the help of some applied cryptography."
                }</p>
                <p>
                    { "The app is packaged as a " }
                    <a href="https://developer.mozilla.org/en-US/docs/WebAssembly/Concepts">
                        { "WASM module" }
                    </a>
                    { ". No data is exchanged with the server during poll operation; all poll data \
                      is stored in the local browser storage and can be backed up if necessary. " }
                    <Link<Route> to={Route::Implementation}>
                        { "More about implementation and security limitations →" }
                    </Link<Route>>
                </p>

                <h4>{ "Polls" }</h4>
                { self.view_polls(ctx) }
            </>
        }
    }
}
