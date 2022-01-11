//! Home page.

use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use super::{common::Icon, Route};
use crate::poll::{PollId, PollManager, PollStage, PollState};

#[derive(Debug)]
pub enum HomeMessage {
    RemovalRequested(PollId),
}

/// Home page component.
#[derive(Debug)]
pub struct Home {
    poll_manager: PollManager,
}

impl Home {
    fn view_polls(&self, ctx: &Context<Self>) -> Html {
        let polls = self.poll_manager.polls();
        let polls: Html = polls
            .into_iter()
            .map(|(id, state)| Self::view_poll(id, &state, ctx))
            .collect();
        html! {
            <>
                { polls }
                <div class="mt-3">
                    <Link<Route> to={Route::NewPoll} classes={classes!["btn", "btn-primary"]}>
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
        html! {
            <div class="card mb-2">
                <div class="card-body">
                    <div class="btn-group btn-group-sm float-end ms-2 mb-2"
                        role="group"
                        aria-label="Actions">

                        <button
                            type="button"
                            class="btn btn-danger"
                            title="Remove this poll"
                            onclick={link.callback(move |_| {
                                HomeMessage::RemovalRequested(id)
                            })}>
                            { Icon::Remove.view() }
                        </button>
                    </div>

                    <h5 class="card-title">{ &state.spec.title }</h5>
                    <p class="card-subtitle mb-2 small text-muted">
                        // FIXME: use real date
                        { "Added on 2022-01-07 13:00:17 UTC" }
                    </p>

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
                    <Link<Route>
                        to={Route::for_poll(id, poll_stage)}
                        classes={classes!["card-link"]}>
                        { "Continue" }
                    </Link<Route>>
                </div>
            </div>
        }
    }

    fn view_poll_stage(stage: PollStage) -> Html {
        match stage {
            PollStage::New => html! { { "Just created" } },
            PollStage::AddingParticipants { participants } => {
                html! {
                    <>
                        { "Adding participants (" }
                        <strong>{ participants.to_string() }</strong>
                        { ")" }
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
                <p class="lead">{ "Welcome!" }</p>
                <p>{ "Lorem ipsum dolor amet. Lorem ipsum dolor amet. Lorem ipsum dolor amet." }</p>
                <h4>{ "Existing polls" }</h4>
                { self.view_polls(ctx) }
            </>
        }
    }
}
