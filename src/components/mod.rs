//! Application.

use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;
use yew::{function_component, html, html::Scope, Callback, Component, Context, Html, Properties};
use yew_router::prelude::*;

mod about;
mod common;
mod new_poll;
mod participants;

use self::{about::About, new_poll::NewPoll, participants::Participants};
use crate::{
    layout,
    poll::{PollId, PollManager, PollSpec},
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
}

/// Application routes.
#[derive(Debug, Clone, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/polls/:id/participants")]
    PollParticipants { id: PollId },
    #[at("/about")]
    About,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[derive(Debug, Clone, Default, PartialEq, Properties)]
pub struct AppProperties {
    /// Callback when a value gets exported.
    #[prop_or_default]
    pub onexport: Callback<ExportedData>,
}

#[derive(Debug)]
pub enum AppMessage {
    PollCreated(PollSpec),
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
                <div class="container pt-4">
                    <main><Main onexport={ctx.props().onexport.clone()} /></main>
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

        match route {
            Route::Home => html! {
                <NewPoll
                    onexport={on_poll_export}
                    ondone={link.callback(AppMessage::PollCreated)} />
            },
            Route::PollParticipants { id } => html! {
                <Participants
                    id={*id}
                    onexport={on_participant_export} />
            },

            Route::About => html! { <About /> },
            Route::NotFound => html! { <NotFound /> },
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
        match msg {
            AppMessage::PollCreated(spec) => {
                let id = self.poll_manager.save_poll(spec);
                let history = ctx.link().history().expect_throw("cannot get history");
                history.replace(Route::PollParticipants { id });
                true
            }
        }
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
