//! Tallying page.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::Event;
use yew::{classes, html, Component, Context, Html};
use yew_router::prelude::*;

use super::{
    common::{view_err, Card, Icon, PageMetadata, PollStageProperties, ValidatedValue},
    Route,
};
use crate::components::common::view_data_row;
use crate::{
    poll::{
        Participant, PollId, PollManager, PollStage, PollState, SubmittedTallierShare, TallierShare,
    },
    utils::{value_from_event, Encode},
};

#[derive(Debug)]
pub enum TallyingMessage {
    ShareSet(String),
    ExportRequested(usize),
}

impl TallyingMessage {
    fn share_set(event: &Event) -> Self {
        Self::ShareSet(value_from_event(event))
    }
}

#[derive(Debug)]
pub struct Tallying {
    metadata: PageMetadata,
    poll_manager: PollManager,
    poll_id: PollId,
    poll_state: Option<PollState>,
    is_readonly: bool,
    new_share: ValidatedValue,
}

impl Tallying {
    fn share(&self, idx: usize) -> Option<&TallierShare> {
        let participants = self.poll_state.as_ref()?.participants();
        Some(&participants.get(idx)?.tallier_share.as_ref()?.inner)
    }

    fn set_share(&mut self, share: String) {
        let parsed_share = match serde_json::from_str::<TallierShare>(&share) {
            Ok(share) => share,
            Err(err) => {
                self.new_share = ValidatedValue {
                    value: share,
                    error_message: Some(format!("Error parsing share: {}", err)),
                };
                return;
            }
        };

        if let Some(state) = &mut self.poll_state {
            if let Err(err) = state.insert_tallier_share(&self.poll_id, parsed_share) {
                self.new_share = ValidatedValue {
                    value: share,
                    error_message: Some(format!("Error verifying share: {}", err)),
                };
                return;
            }
            self.poll_manager.update_poll(&self.poll_id, state);
        }
        self.new_share = ValidatedValue::default();
    }

    fn view_poll(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <p class="lead">{ "The final poll stage is tallying." }</p>
                <p>{ "Each participant must submit a tallying share, which when combined \
                    allow to decrypt the cumulative votes." }</p>

                <h4>{ "Shares" }</h4>
                { self.view_shares(state, ctx) }
            </>
        }
    }

    fn view_shares(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let shares: Html = state
            .participants()
            .iter()
            .enumerate()
            .filter_map(|(idx, participant)| {
                let share = participant.tallier_share.as_ref();
                share.map(|share| {
                    let share = self.view_share(idx, participant, share, ctx);
                    html! { <div class="col-lg-6">{ share }</div> }
                })
            })
            .collect();

        html! {
            <div class="row g-2 mb-3">
                { shares }
                { if self.is_readonly {
                    html!{}
                } else {
                    html! { <div class="col-lg-6">{ self.view_new_share_form(ctx) }</div> }
                }}
            </div>
        }
    }

    fn view_share(
        &self,
        idx: usize,
        participant: &Participant,
        share: &SubmittedTallierShare,
        ctx: &Context<Self>,
    ) -> Html {
        let title = format!("Tallier #{}", idx + 1);
        let mut card = Card::new(
            html! { title },
            html! {
                <p class="card-text mb-0 text-truncate">
                    <strong>{ "Tallier’s public key:" }</strong>
                    { " " }
                    { participant.public_key().encode() }
                </p>
            },
        );

        let our_key = ctx.props().secrets.public_key_for_poll(&self.poll_id);
        if *participant.public_key() == our_key {
            card = card.with_our_mark();
        }

        let link = ctx.link();
        card.with_timestamp(share.submitted_at)
            .with_button(html! {
                <button
                    type="button"
                    class="btn btn-sm btn-secondary"
                    title="Copy share to clipboard"
                    onclick={link.callback(move |_| TallyingMessage::ExportRequested(idx))}>
                    { Icon::Export.view() }{ " Export" }
                </button>
            })
            .view()
    }

    fn view_new_share_form(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_share.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        let card = Card::new(
            html! { <label for="encoded-share">{ "New share" }</label> },
            html! {
                <form>
                    <textarea
                        id="encoded-share"
                        class={control_classes}
                        placeholder="JSON-encoded share"
                        value={self.new_share.value.clone()}
                        onchange={link.callback(|evt| TallyingMessage::share_set(&evt))}>
                    </textarea>
                    { if let Some(err) = &self.new_share.error_message {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </form>
            },
        );
        card.with_dotted_border().view()
    }

    fn view_results(state: &PollState, results: &[u64]) -> Html {
        let total_votes = results.iter().copied().sum::<u64>();
        let options = state.spec().options.iter().zip(results);
        let results: Html = options
            .map(|(option, &votes)| Self::view_option_result(option, votes, total_votes))
            .collect();
        html! {
            <>
                <h4>{ "Vote results" }</h4>
                <h5 class="text-muted">{ &state.spec().title }</h5>
                { if state.spec().description.trim().is_empty() {
                    html!{}
                } else {
                    html! { <p>{ &state.spec().description }</p> }
                }}
                { results }
            </>
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn view_option_result(option: &str, votes: u64, total_votes: u64) -> Html {
        let progress_percent = if total_votes == 0 {
            0.0
        } else {
            votes as f64 * 100.0 / total_votes as f64
        };
        view_data_row(
            html! { <strong>{ option }</strong> },
            html! {
                <>
                    <p class="mb-1">{ format!("{} votes ({:.0}%)", votes, progress_percent) }</p>
                    <div class="progress">
                        <div
                            class="progress-bar"
                            role="progressbar"
                            style={format!("width: {:.2}%", progress_percent)}
                            aria-valuenow={progress_percent.to_string()}
                            aria-valuemin="0"
                            aria-valuemax="100">
                        </div>
                    </div>
                </>
            },
        )
    }
}

impl Component for Tallying {
    type Message = TallyingMessage;
    type Properties = PollStageProperties;

    fn create(ctx: &Context<Self>) -> Self {
        let poll_manager = PollManager::default();
        let poll_id = ctx.props().id;
        let poll_state = poll_manager.poll(&poll_id);
        let is_readonly = poll_state.as_ref().map_or(true, |state| {
            !matches!(state.stage(), PollStage::Tallying { .. })
        });

        Self {
            metadata: PageMetadata {
                title: "Tallying".to_owned(),
                description: "Allows tallying submitted encrypted votes".to_owned(),
                is_root: false,
            },
            poll_manager,
            poll_id,
            poll_state,
            is_readonly,
            new_share: ValidatedValue::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            TallyingMessage::ShareSet(share) => {
                self.set_share(share);
            }
            TallyingMessage::ExportRequested(idx) => {
                if let Some(share) = self.share(idx) {
                    let share = serde_json::to_string_pretty(share)
                        .expect_throw("failed serializing `TallierShare`");
                    ctx.props().onexport.emit(share);
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
                    { self.metadata.view() }
                    { state.stage().view_nav(PollStage::TALLYING_IDX, self.poll_id) }
                    { self.view_poll(state, ctx) }
                    { if let Some(results) = state.results() {
                        Self::view_results(state, results)
                    } else {
                        html!{}
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
