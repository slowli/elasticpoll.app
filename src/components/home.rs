//! Home page.

use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use super::{
    common::{Card, Icon, PageMetadata},
    Route,
};
use crate::poll::{PollId, PollManager, PollStage, PollState};

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
        card.with_timestamp(state.created_at)
            .with_button(html! {
                <Link<Route>
                    to={Route::for_poll(id, poll_stage)}
                    classes={classes!["btn", "btn-sm", "btn-primary", "me-2"]}>
                    { "Continue" }
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
                        { "Adding participants: " }
                        <strong>{ participants.to_string() }</strong>
                    </>
                }
            }
            PollStage::Voting {
                votes,
                participants,
            } => {
                html! {
                    <>
                        { "Voting: " }
                        <strong>{ votes.to_string() }</strong>
                        { " votes / "}
                        <strong>{ participants.to_string() }</strong>
                        { " eligible voters" }
                    </>
                }
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
                <p class="lead">{ "Welcome!" }</p>
                <p>{ "Lorem ipsum dolor amet. Lorem ipsum dolor amet. Lorem ipsum dolor amet." }</p>
                <h4>{ "Existing polls" }</h4>
                { self.view_polls(ctx) }
            </>
        }
    }
}
