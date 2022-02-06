//! Application (the root component).

use wasm_bindgen::UnwrapThrowExt;
use web_sys::Element;
use yew::{html, html::Scope, Callback, Component, Context, ContextProvider, Html, Properties};
use yew_router::prelude::*;

use std::rc::Rc;

use super::{
    about::About, home::Home, implementation::Implementation, new_poll::NewPoll,
    participants::Participants, tallying::Tallying, voting::Voting, NotFound, Route,
};
use crate::{
    js::{ExportedData, ManageModals},
    poll::{PollId, PollManager, PollSpec, PollState, SecretManager, TallierShare},
};

#[derive(Debug, Clone, Properties)]
pub struct AppProperties {
    /// Secrets manager.
    pub secrets: Rc<SecretManager>,
    /// Modal manager.
    pub modals: Rc<dyn ManageModals>,
    /// Callback when a value gets exported.
    #[prop_or_default]
    pub onexport: Callback<(ExportedData, Element)>,
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
    RolledBackToParticipants(PollId, Box<PollState>),
    VotesFinalized(PollId, Box<PollState>),
    RolledBackToVoting(PollId, Box<PollState>),
}

/// Root application component.
#[derive(Debug)]
pub struct App;

impl App {
    fn header() -> Html {
        html! {
            <header class="body-header">
                <div class="container">
                    <div>
                        <h1 class="display-4 mb-0">
                            <Link<Route>
                                to={ Route::Home }
                                classes="d-block">{ "Elastic Poll" }</Link<Route>>
                        </h1>
                        <div class="text-muted">
                            { "Cryptographically secure polling app" }
                        </div>
                    </div>
                </div>
            </header>
        }
    }

    fn footer() -> Html {
        html! {
            <footer class="page-footer small">
                <div class="row">
                    <div class="col-md-9">
                        <img src="/_assets/css/favicon.svg"
                            alt="Site logo"
                            class="float-start me-3 mb-2"
                            width="48"
                            height="48" />
                        <p class="mb-2">
                            { "© 2022 Alex Ostrovski. Licensed under " }
                            <a rel="license" href="https://www.apache.org/licenses/LICENSE-2.0">
                                { "Apache 2.0" }
                            </a>
                        </p>
                        <p class="text-muted">
                            { "This site is open-source! " }
                            <a href="https://github.com/slowli/elasticpoll.app">
                                { "Contribute on GitHub" }
                            </a>
                        </p>
                    </div>
                    <div class="col-md-3">
                        <h5>{ "Useful links" }</h5>
                        <ul class="list-unstyled">
                            <li class="mb-1" title="About this website">
                                <Link<Route> to={Route::Implementation}>
                                    { "Implementation" }
                                </Link<Route>>
                            </li>
                            <li class="mb-1" title="About this website">
                                <Link<Route> to={Route::About}>{ "About" }</Link<Route>>
                            </li>
                            <li>
                                <a href="https://crates.io/crates/elastic-elgamal"
                                    title="Rust library powering this website"
                                    target="_blank">
                                    { "elastic-elgamal library" }
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>
            </footer>
        }
    }
}

impl Component for App {
    type Message = ();
    type Properties = AppProperties;

    fn create(_: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <BrowserRouter>
                { Self::header() }
                <div class="container">
                    <main>
                        <Main
                            secrets={Rc::clone(&ctx.props().secrets)}
                            modals={Rc::clone(&ctx.props().modals)}
                            onexport={ctx.props().onexport.clone()} />
                    </main>
                    { Self::footer() }
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
            Route::Implementation => html! { <Implementation /> },
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
                        })}
                        onrollback={link.callback(move |state| {
                            AppMessage::RolledBackToParticipants(id, Box::new(state))
                        })} />
                }
            }
            Route::Tallying { id } => {
                let id = *id;
                html! {
                    <Tallying
                        id={id}
                        onrollback={link.callback(move |state| {
                            AppMessage::RolledBackToVoting(id, Box::new(state))
                        })} />
                }
            }
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
            AppMessage::RolledBackToParticipants(id, mut state) => {
                state.rollback_to_participants_selection();
                self.poll_manager.update_poll(&id, &state);
                history.push(Route::PollParticipants { id });
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
            AppMessage::RolledBackToVoting(id, mut state) => {
                state.rollback_to_voting();
                self.poll_manager.update_poll(&id, &state);
                history.push(Route::Voting { id });
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
