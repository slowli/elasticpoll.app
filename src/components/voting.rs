//! Voting page.

use wasm_bindgen::UnwrapThrowExt;
use web_sys::{Event, HtmlInputElement};
use yew::{classes, html, Callback, Component, Context, Html, Properties};
use yew_router::prelude::*;

use std::rc::Rc;

use super::{
    common::{view_err, Card, Icon, PageMetadata, ValidatedValue},
    Route,
};
use crate::{
    poll::{
        Participant, PollId, PollManager, PollStage, PollState, SecretManager, SubmittedVote, Vote,
        VoteChoice,
    },
    utils::{get_event_target, value_from_event, Encode},
};

#[derive(Debug)]
pub enum VotingMessage {
    OptionSelected(usize, bool),
    VoteSet(String),
    OurVoteAdded,
    ExportRequested(usize),
}

impl VotingMessage {
    fn option_selected(option_idx: usize, event: &Event) -> Self {
        let target = get_event_target::<HtmlInputElement>(event);
        Self::OptionSelected(option_idx, target.checked())
    }

    fn vote_set(event: &Event) -> Self {
        Self::VoteSet(value_from_event(event))
    }
}

#[derive(Debug, Clone, Properties)]
pub struct VotingProperties {
    pub id: PollId,
    pub secrets: Rc<SecretManager>,
    #[prop_or_default]
    pub onexport: Callback<String>,
}

impl PartialEq for VotingProperties {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && Rc::ptr_eq(&self.secrets, &other.secrets)
            && self.onexport == other.onexport
    }
}

/// Voting page component.
#[derive(Debug)]
pub struct Voting {
    metadata: PageMetadata,
    poll_manager: PollManager,
    poll_id: PollId,
    poll_state: Option<PollState>,
    our_choice: Option<VoteChoice>,
    new_vote: ValidatedValue,
}

impl Voting {
    fn vote(&self, idx: usize) -> Option<&Vote> {
        let participants = self.poll_state.as_ref()?.participants();
        Some(&participants.get(idx)?.vote.as_ref()?.inner)
    }

    fn set_vote(&mut self, vote: String) {
        let parsed_vote = match serde_json::from_str::<Vote>(&vote) {
            Ok(vote) => vote,
            Err(err) => {
                self.new_vote = ValidatedValue {
                    value: vote,
                    error_message: Some(format!("Error parsing vote: {}", err)),
                };
                return;
            }
        };

        if let Some(state) = &mut self.poll_state {
            if let Err(err) = state.insert_vote(&self.poll_id, parsed_vote) {
                self.new_vote = ValidatedValue {
                    value: vote,
                    error_message: Some(format!("Error verifying vote: {}", err)),
                };
                return;
            }
            self.poll_manager.update_poll(&self.poll_id, state);
        }
        self.new_vote = ValidatedValue::default();
    }

    fn insert_our_vote(&mut self, ctx: &Context<Self>) {
        if let Some(state) = &mut self.poll_state {
            if let Some(choice) = &self.our_choice {
                let our_keypair = ctx.props().secrets.keys_for_poll(&self.poll_id);
                let vote = Vote::new(&our_keypair, &self.poll_id, state, choice);
                state.insert_unchecked_vote(vote);
                self.poll_manager.update_poll(&self.poll_id, state);
            }
        }
    }

    fn view_poll(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <p class="lead">{ "After the set of participants is finalized, \
                    voting can commence." }</p>
                <p>{ "Each participant can submit a vote an unlimited number of times." }</p>

                <h4>{ "Votes" }</h4>
                { self.view_votes(state, ctx) }
            </>
        }
    }

    fn view_votes(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        let votes: Html = state
            .participants()
            .iter()
            .enumerate()
            .filter_map(|(idx, participant)| {
                let vote = participant.vote.as_ref();
                vote.map(|vote| {
                    let vote = self.view_vote(idx, participant, vote, ctx);
                    html! { <div class="col-lg-6">{ vote }</div> }
                })
            })
            .collect();
        html! {
            <div class="row g-2 mb-3">
                { votes }
                <div class="col-lg-6">{ self.view_new_vote_form(ctx) }</div>
            </div>
        }
    }

    fn view_vote(
        &self,
        idx: usize,
        participant: &Participant,
        vote: &SubmittedVote,
        ctx: &Context<Self>,
    ) -> Html {
        let title = format!("Voter #{}", idx + 1);
        let mut card = Card::new(
            html! { title },
            html! {
                <>
                    <p class="card-text text-truncate mb-1">
                        <strong>{ "Vote hash:" }</strong>
                        { " " }
                        { &vote.hash }
                    </p>
                    <p class="card-text mb-0 text-truncate">
                        <strong>{ "Voter’s public key:" }</strong>
                        { " " }
                        { participant.public_key().encode() }
                    </p>
                </>
            },
        );

        let our_key = ctx.props().secrets.public_key_for_poll(&self.poll_id);
        if *participant.public_key() == our_key {
            card = card.with_our_mark();
        }

        let link = ctx.link();
        card.with_timestamp(vote.submitted_at)
            .with_button(html! {
                <button
                    type="button"
                    class="btn btn-sm btn-secondary"
                    title="Copy vote to clipboard"
                    onclick={link.callback(move |_| VotingMessage::ExportRequested(idx))}>
                    { Icon::Export.view() }{ " Export" }
                </button>
            })
            .view()
    }

    fn view_new_vote_form(&self, ctx: &Context<Self>) -> Html {
        let mut control_classes = classes!["form-control", "font-monospace", "small", "mb-1"];
        if self.new_vote.error_message.is_some() {
            control_classes.push("is-invalid");
        }

        let link = ctx.link();
        let card = Card::new(
            html! { <label for="encoded-vote">{ "New vote" }</label> },
            html! {
                <form>
                    <textarea
                        id="encoded-vote"
                        class={control_classes}
                        placeholder="JSON-encoded vote"
                        value={self.new_vote.value.clone()}
                        onchange={link.callback(|evt| VotingMessage::vote_set(&evt))}>
                    </textarea>
                    { if let Some(err) = &self.new_vote.error_message {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </form>
            },
        );
        card.with_dotted_border().view()
    }

    fn view_vote_submission(&self, state: &PollState, ctx: &Context<Self>) -> Html {
        if let Some(choice) = &self.our_choice {
            let link = ctx.link();
            let on_change = link.callback(|(idx, evt)| VotingMessage::option_selected(idx, &evt));
            let card = Card::new(
                html! { &state.spec().title },
                state.spec().view_as_form(choice, on_change),
            );

            card.with_button(html! {
                <button
                    type="button"
                    class="btn btn-sm btn-primary"
                    onclick={link.callback(|_| VotingMessage::OurVoteAdded)}>
                    { Icon::Plus.view() }{ " Add your vote" }
                </button>
            })
            .view()
        } else {
            html! {
                <>
                    <div class="alert alert-warning" role="alert">
                        { "You are not a poll participant and cannot vote in this poll." }
                    </div>
                    { state.spec().view_summary_card() }
                </>
            }
        }
    }
}

impl Component for Voting {
    type Message = VotingMessage;
    type Properties = VotingProperties;

    fn create(ctx: &Context<Self>) -> Self {
        let poll_manager = PollManager::default();
        let poll_id = ctx.props().id;
        let poll_state = poll_manager
            .poll(&poll_id)
            .filter(|state| matches!(state.stage(), PollStage::Voting { .. }));

        let our_key = ctx.props().secrets.public_key_for_poll(&poll_id);
        let our_choice = poll_state.as_ref().and_then(|state| {
            let we_are_participant = state
                .participants()
                .iter()
                .any(|p| *p.public_key() == our_key);
            if we_are_participant {
                Some(VoteChoice::default(state.spec()))
            } else {
                None
            }
        });

        Self {
            metadata: PageMetadata {
                title: "Voting & vote management".to_owned(),
                description: "Allows creating and submitting votes for the poll".to_owned(),
                is_root: false,
            },
            poll_manager,
            poll_id,
            poll_state,
            our_choice,
            new_vote: ValidatedValue::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            VotingMessage::OptionSelected(option_idx, selected) => {
                if let Some(choice) = &mut self.our_choice {
                    choice.select(option_idx, selected);
                }
            }
            VotingMessage::VoteSet(vote) => {
                self.set_vote(vote);
            }
            VotingMessage::OurVoteAdded => {
                self.insert_our_vote(ctx);
            }
            VotingMessage::ExportRequested(idx) => {
                if let Some(vote) = self.vote(idx) {
                    let vote = serde_json::to_string_pretty(vote)
                        .expect_throw("failed serializing `Vote`");
                    ctx.props().onexport.emit(vote);
                }
                return false;
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if let Some(state) = &self.poll_state {
            let no_votes = !state.contains_votes();
            html! {
                <>
                    { self.metadata.view() }
                    { state.stage().view_nav(PollStage::VOTING_IDX, self.poll_id) }
                    { self.view_poll(state, ctx) }

                    <h4>{ "Submit vote" }</h4>
                    { self.view_vote_submission(state, ctx) }

                    <div class="mt-4 text-center">
                        <button
                            type="button"
                            class="btn btn-primary"
                            disabled={no_votes}>
                            { Icon::Check.view() }{ " Next: tallying" }
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
