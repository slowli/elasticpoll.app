//! Application page components.

use wasm_bindgen::{JsValue, UnwrapThrowExt};
use yew::{classes, function_component, html, Callback, Html, Properties};
use yew_router::prelude::*;

mod about;
mod app;
mod home;
mod implementation;
mod new_poll;
mod participants;
mod tallying;
mod voting;

pub use self::{
    app::{App, AppProperties},
    new_poll::{NewPoll, NewPollMessage, NewPollProperties},
};

use crate::poll::{PollId, PollStage, PollState};

/// Application routes.
#[derive(Debug, Clone, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/about")]
    About,
    #[at("/implementation")]
    Implementation,

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

impl PollStage {
    /// Renders navigation for poll stages with the current stage selected.
    pub fn view_nav(&self, active_idx: usize, id: PollId) -> Html {
        debug_assert!(self.index() >= active_idx);
        html! {
            <ul class="nav mb-3 nav-pills flex-column flex-md-row justify-content-md-center">
                <li class="nav-item">
                    <a class="nav-link disabled">{ "1. Specification" }</a>
                </li>
                { self.view_nav_item(
                    1,
                    active_idx,
                    Route::PollParticipants { id },
                    "2. Participants",
                ) }
                { self.view_nav_item(
                    2,
                    active_idx,
                    Route::Voting { id },
                    "3. Voting",
                ) }
                { self.view_nav_item(
                    3,
                    active_idx,
                    Route::Tallying { id },
                    "4. Tallying",
                ) }
            </ul>
        }
    }

    fn view_nav_item(&self, idx: usize, active_idx: usize, route: Route, name: &str) -> Html {
        html! {
            <li class="nav-item">
                { if self.index() >= idx {
                    let mut link_classes = classes!["nav-link"];
                    if active_idx == idx {
                        link_classes.push("active");
                    }
                    html! {
                        <Link<Route> to={route} classes={link_classes}>{ name }</Link<Route>>
                    }
                } else {
                    html! { <a class="nav-link disabled">{ name }</a> }
                }}
            </li>
        }
    }
}

/// Component responsible for rendering page metadata via a portal.
#[derive(Debug)]
pub struct PageMetadata {
    pub title: String,
    pub description: String,
    pub is_root: bool,
}

impl PageMetadata {
    pub fn view(&self) -> Html {
        let window = web_sys::window().expect_throw("no Window");
        let document = window.document().expect_throw("no Document");
        let head = document.head().expect_throw("no <head> in Document");
        let prerender = JsValue::from_str("__PRERENDER__").js_in(&window);

        if !prerender {
            // Remove prerendered versions of the attributes. If this is not the first
            // rendering of page meta, this is a no-op.
            let prerendered_elements = head.query_selector_all(".prerender").unwrap_throw();
            for i in 0..prerendered_elements.length() {
                if let Some(node) = prerendered_elements.item(i) {
                    head.remove_child(&node).ok();
                }
            }
        }
        yew::create_portal(self.view_meta(prerender), head.into())
    }

    fn view_meta(&self, prerender: bool) -> Html {
        let element_classes = if prerender {
            classes!["prerender"]
        } else {
            classes![]
        };
        html! {
            <>
                <meta
                    class={element_classes.clone()}
                    name="description"
                    content={self.description.clone()} />
                <meta
                    class={element_classes.clone()}
                    name="og:title"
                    content={self.title.clone()} />
                <meta
                    class={element_classes.clone()}
                    name="og:description"
                    content={self.description.clone()} />
                <script class={element_classes.clone()} type="application/ld+json">
                    { self.linked_data() }
                </script>
                <title class={element_classes}>{ &self.title }{ " | Elastic Poll" }</title>
            </>
        }
    }

    fn linked_data(&self) -> String {
        format!(
            "{{\
                \"@context\":\"https://schema.org/\",\
                \"@type\":\"{ty}\",\
                \"author\":{{\
                  \"@type\":\"Person\",\
                  \"name\":\"Alex Ostrovski\"\
                }},\
                \"headline\":\"{title}\",\
                \"description\":\"{description}\"\
            }}",
            ty = if self.is_root { "WebSite" } else { "WebPage" },
            title = self.title,
            description = self.description
        )
    }
}

/// Properties for vote stage pages.
#[derive(Debug, Clone, PartialEq, Properties)]
pub struct PollStageProperties {
    pub id: PollId,
    #[prop_or_default]
    pub ondone: Callback<PollState>,
    #[prop_or_default]
    pub onrollback: Callback<PollState>,
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
