//! [`PollManager`] and [`SecretsManager`].

use js_sys::{Error, JsString, Uint8Array};
use rand_core::OsRng;
use secret_tree::{SecretTree, Seed};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;

use std::{cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc, str::FromStr};

use super::{Keypair, PollId, PollSpec, PollState, PublicKey};
use crate::{js::PasswordBasedCrypto, utils::local_storage};

#[derive(Debug)]
pub struct PollManager {
    storage_key_prefix: &'static str,
}

impl Default for PollManager {
    fn default() -> Self {
        Self {
            storage_key_prefix: "elastic_poll",
        }
    }
}

impl PollManager {
    /// Returns ID of the saved poll.
    pub fn create_poll(&mut self, spec: PollSpec) -> PollId {
        let id = PollId::for_spec(&spec);
        let local_storage = local_storage();
        let poll = PollState::new(spec);
        let poll = serde_json::to_string(&poll).expect_throw("cannot serialize `PollState`");
        let key = format!("{}::poll::{id}", self.storage_key_prefix);
        local_storage
            .set_item(&key, &poll)
            .expect_throw("failed saving poll");
        id
    }

    /// Lists polls together with the respective IDs.
    pub fn polls(&self) -> Vec<(PollId, PollState)> {
        let local_storage = local_storage();
        // This iteration protocol assumes that the storage is not modified concurrently.
        let len = local_storage
            .length()
            .expect_throw("cannot obtain local storage length");
        let polls = (0..len).filter_map(|idx| {
            let key = local_storage
                .key(idx)
                .expect_throw("cannot obtain key from storage")?;
            self.extract_poll_id(&key).and_then(|poll_id| {
                let state_string = local_storage
                    .get_item(&key)
                    .expect_throw("failed getting poll state")?;
                let state = serde_json::from_str(&state_string).ok()?;
                Some((poll_id, state))
            })
        });
        polls.collect()
    }

    fn extract_poll_id(&self, storage_key: &str) -> Option<PollId> {
        if !storage_key.starts_with(self.storage_key_prefix) {
            return None;
        }
        let key_tail = &storage_key[self.storage_key_prefix.len()..];
        if !key_tail.starts_with("::poll::") {
            return None;
        }
        let key_tail = &key_tail[8..]; // "::poll::".len() == 8
        PollId::from_str(key_tail).ok()
    }

    /// Gets the poll state by ID.
    pub fn poll(&self, id: &PollId) -> Option<PollState> {
        let local_storage = local_storage();
        let key = format!("{}::poll::{id}", self.storage_key_prefix);
        let state_string = local_storage
            .get_item(&key)
            .expect_throw("failed getting poll state")?;
        serde_json::from_str(&state_string).ok()
    }

    // TODO: CAS semantics?
    pub fn update_poll(&self, id: &PollId, poll: &PollState) {
        let local_storage = local_storage();
        let key = format!("{}::poll::{id}", self.storage_key_prefix);
        let poll = serde_json::to_string(&poll).expect_throw("cannot serialize `PollState`");
        local_storage
            .set_item(&key, &poll)
            .expect_throw("failed saving poll");
    }

    pub fn remove_poll(&self, id: &PollId) {
        let local_storage = local_storage();
        let key = format!("{}::poll::{id}", self.storage_key_prefix);
        local_storage
            .remove_item(&key)
            .expect_throw("cannot remove `PollState` from local storage");
    }
}

#[derive(Debug)]
enum SecretManagerState {
    Locked,
    Unlocked(SecretTree),
}

impl Default for SecretManagerState {
    fn default() -> Self {
        Self::Locked
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretManagerStatus {
    Locked,
    Unlocked,
}

/// Manager of application secrets.
#[derive(Debug)]
pub struct SecretManager {
    storage_key: &'static str,
    state: RefCell<SecretManagerState>,
    pk_cache: RefCell<HashMap<PollId, PublicKey>>,
    crypto: Rc<dyn PasswordBasedCrypto>,
}

impl SecretManager {
    pub fn new(crypto: Rc<dyn PasswordBasedCrypto>) -> Self {
        Self {
            storage_key: "elastic_poll::secret",
            state: RefCell::default(),
            pk_cache: RefCell::default(),
            crypto,
        }
    }

    fn persist(&self, box_json: &str) {
        local_storage()
            .set_item(self.storage_key, box_json)
            .expect_throw("cannot persist encrypted secret");
    }

    fn encrypted_secret(&self) -> Option<String> {
        local_storage()
            .get_item(self.storage_key)
            .expect_throw("failed getting encrypted secret")
    }

    fn unlock_with_secret(&self, secret: SecretTree) {
        *self.state.borrow_mut() = SecretManagerState::Unlocked(secret);
    }

    pub fn status(&self) -> Option<SecretManagerStatus> {
        if self.encrypted_secret().is_none() {
            None
        } else {
            Some(match *self.state.borrow() {
                SecretManagerState::Locked => SecretManagerStatus::Locked,
                SecretManagerState::Unlocked(_) => SecretManagerStatus::Unlocked,
            })
        }
    }

    /// Returns `true` if the load was successful and `false` otherwise.
    pub fn try_load_cached(self: &Rc<Self>) -> impl Future<Output = bool> {
        let task = self.crypto.cached();
        let this = Rc::clone(self);
        async move {
            if let Ok(maybe_secret_bytes) = JsFuture::from(task).await {
                if maybe_secret_bytes.is_falsy() {
                    return false; // no cached value
                }
                let secret_bytes = maybe_secret_bytes
                    .dyn_into::<Uint8Array>()
                    .expect_throw("unexpected cached output");
                let mut seed = [0_u8; 32];
                secret_bytes.copy_to(&mut seed);
                this.unlock_with_secret(SecretTree::from_seed(Seed::from(&seed)));
                true
            } else {
                // TODO: log errors?
                false
            }
        }
    }

    pub fn encrypt_new_secret(
        self: &Rc<Self>,
        password: &str,
    ) -> impl Future<Output = Result<(), Error>> {
        // We use pinning to enable to pass a ref `&[u8]` of the seed to the host
        // (i.e., not copying seed bytes to a `Box<[u8]>`. If pinning is not used,
        // the seed is moved to the closure and will lead to `seal` encrypting garbage
        // instead of the seed.
        let secret = Box::pin(SecretTree::new(&mut OsRng));
        let task = self.crypto.seal(password, secret.seed().expose_secret());

        let this = Rc::clone(self);
        async move {
            JsFuture::from(task)
                .await
                .map(|box_json| {
                    let box_json = box_json
                        .dyn_into::<JsString>()
                        .expect_throw("unexpected seal_fn output");
                    this.persist(&String::from(box_json));
                    this.unlock_with_secret(*Pin::into_inner(secret));
                })
                .map_err(|err| {
                    err.dyn_into::<Error>()
                        .unwrap_or_else(|_| Error::new("(unknown error)"))
                })
        }
    }

    pub fn unlock(self: &Rc<Self>, password: &str) -> impl Future<Output = Result<(), Error>> {
        let encrypted_secret = self
            .encrypted_secret()
            .expect_throw("called `unlock` without stored secret");
        let task = self.crypto.open(password, &encrypted_secret);

        let this = Rc::clone(self);
        async move {
            JsFuture::from(task)
                .await
                .map(|secret_bytes| {
                    let secret_bytes = secret_bytes
                        .dyn_into::<Uint8Array>()
                        .expect_throw("unexpected open_fn output");
                    let mut seed = [0_u8; 32];
                    secret_bytes.copy_to(&mut seed);
                    this.unlock_with_secret(SecretTree::from_seed(Seed::from(&seed)));
                })
                .map_err(|err| {
                    err.dyn_into::<Error>()
                        .unwrap_or_else(|_| Error::new("(unknown error)"))
                })
        }
    }

    /// Returns `None` if the manager is not unlocked.
    pub fn keys_for_poll(&self, poll_id: &PollId) -> Option<Keypair> {
        let state = self.state.borrow();
        let secret = match &*state {
            SecretManagerState::Unlocked(tree) => tree,
            SecretManagerState::Locked => return None,
        };

        let child = secret.digest(&poll_id.0);
        let keypair = Keypair::generate(&mut child.rng());
        self.pk_cache
            .borrow_mut()
            .insert(*poll_id, keypair.public().clone());
        Some(keypair)
    }

    /// Returns `None` if the manager is not unlocked.
    pub fn public_key_for_poll(&self, poll_id: &PollId) -> Option<PublicKey> {
        let pk = self.pk_cache.borrow().get(poll_id).cloned();
        pk.or_else(|| self.keys_for_poll(poll_id).map(|keys| keys.into_tuple().0))
    }
}
