//! Secrets dialog.

use js_sys::Error;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::{FocusEvent, HtmlInputElement};
use yew::{classes, html, Component, Context, Html, NodeRef};

use super::{common::view_err, AppProperties};
use crate::poll::{SecretManager, SecretManagerStatus};

#[derive(Debug)]
pub enum SecretsMessage {
    Created,
    Unlocked,
    ErrorUnlocking(Error),
    Submitted { new_secret: bool },
}

#[derive(Debug)]
pub struct Secrets {
    input_ref: NodeRef,
    in_progress: bool,
    new_secret: bool,
    err: Option<String>,
}

impl Secrets {
    fn password(&self) -> String {
        self.input_ref
            .cast::<HtmlInputElement>()
            .expect_throw("failed downcasting password input")
            .value()
    }

    pub fn view_alert(secrets: &SecretManager, item: &str) -> Html {
        let (alert_text, button_caption) = match secrets.status() {
            Some(SecretManagerStatus::Locked) => (
                format!(
                    "The secret is locked. Unlock to find out whether a {} was submitted by you, \
                     or to submit a new {0}.",
                    item
                ),
                "Unlock",
            ),
            Some(SecretManagerStatus::Unlocked) => return html! {},
            None => (
                format!("No secret. Create a secret to submit a {}.", item),
                "Create secret",
            ),
        };
        html! {
            <div class="alert alert-warning py-2" role="alert">
                { alert_text }
                <button
                    type="button"
                    class="btn btn-sm btn-primary align-baseline ms-2"
                    data-bs-toggle="modal"
                    data-bs-target="#unlock-secrets-modal">
                    { button_caption }
                </button>
            </div>
        }
    }

    fn view_form(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let new_secret = self.new_secret;
        let button_caption = if new_secret {
            "Create secret"
        } else {
            "Unlock"
        };
        let mut input_classes = classes!["form-control"];
        if self.err.is_some() {
            input_classes.push("is-invalid");
        }

        html! {
            <form onsubmit={link.callback(move |evt: FocusEvent| {
                evt.prevent_default();
                SecretsMessage::Submitted { new_secret }
            })}>
                <div class="modal-body">
                    <label for="password-input" class="form-label">{ "Password" }</label>
                    <input
                        ref={self.input_ref.clone()}
                        type="password"
                        id="password-input"
                        class={input_classes}
                        placeholder="Password to unlock the secret"
                        disabled={self.in_progress} />
                    { if let Some(err) = &self.err {
                        view_err(err)
                    } else {
                        html!{}
                    }}
                </div>
                <div class="modal-footer">
                    <button
                        type="submit"
                        class="btn btn-primary"
                        disabled={self.in_progress}>
                        { button_caption }
                    </button>
                </div>
            </form>
        }
    }
}

impl Component for Secrets {
    type Message = SecretsMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let secrets = &AppProperties::from_ctx(ctx).secrets;
        let new_secret = !matches!(secrets.status(), Some(SecretManagerStatus::Locked));
        Self {
            input_ref: NodeRef::default(),
            new_secret,
            in_progress: false,
            err: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let global_props = AppProperties::from_ctx(ctx);
        match msg {
            SecretsMessage::Submitted { new_secret } => {
                let password = self.password();
                let link = ctx.link().clone();
                let secrets = &global_props.secrets;
                if new_secret {
                    let task = secrets.encrypt_new_secret(&password);
                    spawn_local(async move {
                        match task.await {
                            Ok(()) => link.send_message(SecretsMessage::Created),
                            Err(err) => link.send_message(SecretsMessage::ErrorUnlocking(err)),
                        }
                    });
                } else {
                    let task = secrets.unlock(&password);
                    spawn_local(async move {
                        match task.await {
                            Ok(()) => link.send_message(SecretsMessage::Unlocked),
                            Err(err) => link.send_message(SecretsMessage::ErrorUnlocking(err)),
                        }
                    });
                }
                self.in_progress = true;
                return false;
            }
            SecretsMessage::Created | SecretsMessage::Unlocked => {
                self.in_progress = false;
                self.err = None;
                global_props.modals.hide_modal("unlock-secrets-modal");
            }
            SecretsMessage::ErrorUnlocking(err) => {
                self.in_progress = false;
                self.err = Some(err.message().into());
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="unlock-secrets-modal"
                class="modal"
                tabindex="-1"
                aria-labelledby="unlock-secrets-modal-label"
                aria-hidden="true">

                <div class="modal-dialog">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h5 id="unlock-secrets-modal-label" class="modal-title">
                                { "Unlock secret" }
                            </h5>
                            <button
                                type="button"
                                class="btn-close"
                                data-bs-dismiss="modal"
                                aria-label="Close">
                            </button>
                        </div>
                        { self.view_form(ctx) }
                    </div>
                </div>
            </div>
        }
    }
}
