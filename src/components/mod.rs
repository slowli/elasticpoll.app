//! Application.

use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;
use yew::{function_component, html, html::Scope, Callback, Component, Context, Html, Properties};
use yew_router::prelude::*;

use std::rc::Rc;

mod about;
mod common;
mod home;
mod new_poll;
mod participants;
mod tallying;
mod voting;

use self::{
    about::About, home::Home, new_poll::NewPoll, participants::Participants, tallying::Tallying,
    voting::Voting,
};
use crate::{
    layout,
    poll::{PollId, PollManager, PollSpec, PollStage, PollState, SecretManager, TallierShare},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedData {
    #[serde(rename = "type")]
    ty: ExportedDataType,
    data: String,
}

impl ExportedData {
    pub fn new(ty: ExportedDataType, data: String) -> Self {
        Self { ty, data }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportedDataType {
    PollSpec,
    Application,
    Vote,
    TallierShare,
}

/// Application routes.
#[derive(Debug, Clone, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/about")]
    About,

    #[at("/polls/new")]
    NewPoll,
    #[at("/polls/:id/participants")]
    PollParticipants { id: PollId },
    #[at("/polls/:id/vote")]
    Voting { id: PollId },
    #[at("/polls/:id/tally")]
    Tallying { id: PollId },

    #[not_found]
    #[at("/404")]
    NotFound,
}

impl Route {
    pub fn for_poll(id: PollId, stage: PollStage) -> Self {
        match stage {
            PollStage::Participants { .. } => Self::PollParticipants { id },
            PollStage::Voting { .. } => Self::Voting { id },
            PollStage::Tallying { .. } | PollStage::Finished => Self::Tallying { id },
        }
    }
}

#[derive(Debug, Clone, Default, Properties)]
pub struct AppProperties {
    /// Secrets manager.
    #[prop_or_default]
    pub secrets: Rc<SecretManager>,
    /// Callback when a value gets exported.
    #[prop_or_default]
    pub onexport: Callback<ExportedData>,
}

impl PartialEq for AppProperties {
    fn eq(&self, other: &Self) -> bool {
        self.onexport == other.onexport && Rc::ptr_eq(&self.secrets, &other.secrets)
    }
}

#[derive(Debug)]
pub enum AppMessage {
    PollCreated(PollSpec),
    ParticipantsFinalized(PollId, Box<PollState>),
    VotesFinalized(PollId, Box<PollState>),
}

/// Root application component.
#[derive(Debug)]
pub struct App;

impl Component for App {
    type Message = ();
    type Properties = AppProperties;

    fn create(_: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                { layout::header() }
                <div class="container">
                    <main>
                        <Main
                            secrets={Rc::clone(&ctx.props().secrets)}
                            onexport={ctx.props().onexport.clone()} />
                    </main>
                    { layout::footer() }
                </div>
            </BrowserRouter>
        }
    }
}

/// Component rendering the main view.
///
/// We need this as a separate component because otherwise `ctx.link().history()` won't work
/// (the component needs to be embedded within a `BrowserRouter`).
#[derive(Debug)]
struct Main {
    poll_manager: PollManager,
}

impl Main {
    fn render_route(route: &Route, props: &AppProperties, link: &Scope<Self>) -> Html {
        let on_poll_export = props
            .onexport
            .reform(|data| ExportedData::new(ExportedDataType::PollSpec, data));
        let on_participant_export = props
            .onexport
            .reform(|data| ExportedData::new(ExportedDataType::Application, data));
        let on_vote_export = props
            .onexport
            .reform(|data| ExportedData::new(ExportedDataType::Vote, data));
        let on_share_export = props
            .onexport
            .reform(|data| ExportedData::new(ExportedDataType::TallierShare, data));

        match route {
            Route::Home => html! { <Home /> },
            Route::About => html! { <About /> },
            Route::NotFound => html! { <NotFound /> },

            Route::NewPoll => html! {
                <NewPoll
                    onexport={on_poll_export}
                    ondone={link.callback(AppMessage::PollCreated)} />
            },
            Route::PollParticipants { id } => {
                let id = *id;
                html! {
                    <Participants
                        id={id}
                        secrets={Rc::clone(&props.secrets)}
                        onexport={on_participant_export}
                        ondone={link.callback(move |state| {
                            AppMessage::ParticipantsFinalized(id, Box::new(state))
                        })} />
                }
            }
            Route::Voting { id } => {
                let id = *id;
                html! {
                    <Voting
                        id={id}
                        secrets={Rc::clone(&props.secrets)}
                        onexport={on_vote_export}
                        ondone={link.callback(move |state| {
                            AppMessage::VotesFinalized(id, Box::new(state))
                        })} />
                }
            }
            Route::Tallying { id } => html! {
                <Tallying
                    id={*id}
                    secrets={Rc::clone(&props.secrets)}
                    onexport={on_share_export} />
            },
        }
    }
}

impl Component for Main {
    type Message = AppMessage;
    type Properties = AppProperties;

    fn create(_: &Context<Self>) -> Self {
        Self {
            poll_manager: PollManager::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let history = ctx.link().history().expect_throw("cannot get history");
        match msg {
            AppMessage::PollCreated(spec) => {
                let id = self.poll_manager.create_poll(spec);
                history.replace(Route::PollParticipants { id });
            }
            AppMessage::ParticipantsFinalized(id, mut state) => {
                state.finalize_participants();
                self.poll_manager.update_poll(&id, &state);
                history.push(Route::Voting { id });
            }
            AppMessage::VotesFinalized(id, mut state) => {
                state.finalize_votes();
                let our_keys = ctx.props().secrets.keys_for_poll(&id);

                let we_are_participant = state
                    .participants()
                    .iter()
                    .any(|p| p.public_key() == our_keys.public());
                if we_are_participant {
                    let share = TallierShare::new(&our_keys, &id, &state);
                    state.insert_unchecked_tallier_share(share);
                }

                self.poll_manager.update_poll(&id, &state);
                history.push(Route::Tallying { id });
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props().clone();
        let link = ctx.link().clone();
        let render = Switch::render(move |route| Self::render_route(route, &props, &link));

        html! {
            <Switch<Route> render={render} />
        }
    }
}

#[function_component(NotFound)]
fn not_found_page() -> Html {
    html! {
        <>
            <h3 class="display-5 mb-4">{ "This route does not exist :(" }</h3>
            <p>
                { "Perhaps, navigating back to " }
                <Link<Route> to={ Route::Home }>{ "the main page" }</Link<Route>>
                { " could help." }
            </p>
        </>
    }
}
