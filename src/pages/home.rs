//! Home page.

use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use crate::{
    layout::{Card, Icon},
    pages::{PageMetadata, Route},
    poll::{PollId, PollManager, PollStage, PollState},
};

#[derive(Debug)]
pub enum HomeMessage {
    RemovalRequested(PollId),
}

/// Home page component.
#[derive(Debug)]
pub struct Home {
    poll_manager: PollManager,
    metadata: PageMetadata,
}

impl Home {
    fn view_polls(&self, ctx: &Context<Self>) -> Html {
        let polls = self.poll_manager.polls();
        let polls: Html = polls
            .into_iter()
            .map(|(id, state)| {
                html! { <div class="col-lg-6">{ Self::view_poll(id, &state, ctx) }</div> }
            })
            .collect();
        html! {
            <>
                <div class="row g-2 mb-2">
                    { polls }
                </div>
                <div>
                    <Link<Route>
                        to={Route::NewPoll}
                        classes={classes!["btn", "btn-outline-primary"]}>
                        {Icon::Plus.view()}{ " Create new poll" }
                    </Link<Route>>
                </div>
            </>
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn view_poll(id: PollId, state: &PollState, ctx: &Context<Self>) -> Html {
        let poll_stage = state.stage();
        let progress_percent = (poll_stage.index() as f64 / PollStage::MAX_INDEX as f64) * 100.0;

        let link = ctx.link();
        let card = Card::new(
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

        let continue_text = if matches!(poll_stage, PollStage::Finished) {
            "View results"
        } else {
            "Continue"
        };
        card.with_timestamp(state.created_at)
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
                    class="btn btn-sm btn-danger"
                    title="Remove this poll"
                    onclick={link.callback(move |_| HomeMessage::RemovalRequested(id))}>
                    { Icon::Remove.view() }{ " Remove" }
                </button>
            })
            .view()
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
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            HomeMessage::RemovalRequested(id) => {
                // FIXME: confirm via dialog or toast
                self.poll_manager.remove_poll(&id);
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
