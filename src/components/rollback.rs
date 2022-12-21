//! Rollback modal.

use web_sys::SubmitEvent;
use yew::{html, Callback, Component, Context, Html, Properties};

use crate::{layout::Icon, pages::AppProperties};

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct RollbackProperties {
    pub removed_entities: &'static str,
    pub changed_entities: &'static str,
    pub onconfirmed: Callback<()>,
}

#[derive(Debug)]
pub enum RollbackMessage {
    Confirmed,
}

#[derive(Debug)]
pub struct Rollback;

impl Rollback {
    pub const MODAL_ID: &'static str = "rollback-confirmation-modal";

    fn view_form(ctx: &Context<Self>) -> Html {
        let RollbackProperties {
            removed_entities,
            changed_entities,
            ..
        } = ctx.props();
        let link = ctx.link();

        html! {
            <form onsubmit={link.callback(move |evt: SubmitEvent| {
                evt.prevent_default();
                RollbackMessage::Confirmed
            })}>
                <div class="modal-body">
                    <p>{ "Rolling back will remove all " }
                    { *removed_entities }
                    { " associated with the poll since they will be invalid after changing " }
                    { *changed_entities }
                    { "." }</p>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">
                        { "Close" }
                    </button>
                    <button type="submit" class="btn btn-danger">
                        { Icon::Reset.view() }{ " Rollback" }
                    </button>
                </div>
            </form>
        }
    }
}

impl Component for Rollback {
    type Message = RollbackMessage;
    type Properties = RollbackProperties;

    fn create(_: &Context<Self>) -> Self {
        Self
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            RollbackMessage::Confirmed => {
                ctx.props().onconfirmed.emit(());
                AppProperties::from_ctx(ctx)
                    .modals
                    .hide_modal(Self::MODAL_ID);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id={Self::MODAL_ID}
                class="modal"
                tabindex="-1"
                aria-labelledby="rollback-confirmation-modal-label"
                aria-hidden="true">

                <div class="modal-dialog">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h5 id="rollback-confirmation-modal-label" class="modal-title">
                                { "Rollback poll?" }
                            </h5>
                            <button
                                type="button"
                                class="btn-close"
                                data-bs-dismiss="modal"
                                aria-label="Close">
                            </button>
                        </div>
                        { Self::view_form(ctx) }
                    </div>
                </div>
            </div>
        }
    }
}
