//! Application.

use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;
use yew::{
    function_component, html, html::Scope, Callback, Component, Context, ContextProvider, Html,
    Properties,
};
use yew_router::prelude::*;

use std::rc::Rc;

mod about;
mod common;
mod home;
mod new_poll;
mod participants;
mod secrets;
mod tallying;
mod voting;

use self::{
    about::About, home::Home, new_poll::NewPoll, participants::Participants, tallying::Tallying,
    voting::Voting,
};
use crate::{
    layout,
    poll::{PollId, PollManager, PollSpec, PollStage, PollState, SecretManager, TallierShare},
    ManageModals,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedData {
    #[serde(rename = "type")]
    ty: ExportedDataType,
    data: String,
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

#[derive(Debug, Clone, Properties)]
pub struct AppProperties {
    /// Secrets manager.
    pub secrets: Rc<SecretManager>,
    /// Modal manager.
    pub modals: Rc<dyn ManageModals>,
    /// Callback when a value gets exported.
    #[prop_or_default]
    pub onexport: Callback<ExportedData>,
}

impl PartialEq for AppProperties {
    fn eq(&self, other: &Self) -> bool {
        self.onexport == other.onexport && Rc::ptr_eq(&self.secrets, &other.secrets)
    }
}

impl AppProperties {
    pub fn from_ctx<C: Component>(ctx: &Context<C>) -> Self {
        let (this, _) = ctx
            .link()
            .context::<Self>(Callback::noop())
            .expect_throw("no `AppProperties` context");
        this
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
                            modals={Rc::clone(&ctx.props().modals)}
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
    fn render_route(route: &Route, link: &Scope<Self>) -> Html {
        match route {
            Route::Home => html! { <Home /> },
            Route::About => html! { <About /> },
            Route::NotFound => html! { <NotFound /> },

            Route::NewPoll => html! {
                <NewPoll ondone={link.callback(AppMessage::PollCreated)} />
            },
            Route::PollParticipants { id } => {
                let id = *id;
                html! {
                    <Participants
                        id={id}
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
                        ondone={link.callback(move |state| {
                            AppMessage::VotesFinalized(id, Box::new(state))
                        })} />
                }
            }
            Route::Tallying { id } => html! {
                <Tallying id={*id} />
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
                if let Some(our_keys) = our_keys {
                    if state.has_participant(our_keys.public()) {
                        let share = TallierShare::new(&our_keys, &id, &state);
                        state.insert_unchecked_tallier_share(share);
                    }
                }
                self.poll_manager.update_poll(&id, &state);
                history.push(Route::Tallying { id });
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();
        let render = Switch::render(move |route| Self::render_route(route, &link));

        html! {
            <ContextProvider<AppProperties> context={ctx.props().clone()}>
                <Switch<Route> render={render} />
            </ContextProvider<AppProperties>>
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
