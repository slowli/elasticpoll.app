//! [`PollManager`] and [`SecretsManager`].

use elastic_elgamal::{group::Ristretto, Keypair, PublicKey};
use rand_core::OsRng;
use secret_tree::SecretTree;
use wasm_bindgen::UnwrapThrowExt;

use std::{cell::RefCell, collections::HashMap, str::FromStr};

use super::{PollId, PollSpec, PollState};
use crate::utils::local_storage;

#[derive(Debug)]
pub struct PollManager {
    storage_key_prefix: &'static str,
}

impl Default for PollManager {
    fn default() -> Self {
        Self {
            storage_key_prefix: "elastic_elgamal_site", // FIXME
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
        let key = format!("{}::poll::{}", self.storage_key_prefix, id);
        local_storage
            .set_item(&key, &poll)
            .expect_throw("failed saving poll");
        id
    }

    /// Lists polls together with the respective IDs.
    pub fn polls(&self) -> HashMap<PollId, PollState> {
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
        let key = format!("{}::poll::{}", self.storage_key_prefix, id);
        let state_string = local_storage
            .get_item(&key)
            .expect_throw("failed getting poll state")?;
        serde_json::from_str(&state_string).ok()
    }

    // TODO: CAS semantics?
    pub fn update_poll(&self, id: &PollId, poll: &PollState) {
        let local_storage = local_storage();
        let key = format!("{}::poll::{}", self.storage_key_prefix, id);
        let poll = serde_json::to_string(&poll).expect_throw("cannot serialize `PollState`");
        local_storage
            .set_item(&key, &poll)
            .expect_throw("failed saving poll");
    }

    pub fn remove_poll(&self, id: &PollId) {
        let local_storage = local_storage();
        let key = format!("{}::poll::{}", self.storage_key_prefix, id);
        local_storage
            .remove_item(&key)
            .expect_throw("cannot remove `PollState` from local storage");
    }
}

/// Manager of application secrets.
// FIXME: store in local storage in password-encrypted form; query password to unlock
#[derive(Debug)]
pub struct SecretManager {
    secret: SecretTree,
    pk_cache: RefCell<HashMap<PollId, PublicKey<Ristretto>>>,
}

impl Default for SecretManager {
    fn default() -> Self {
        Self {
            secret: SecretTree::new(&mut OsRng),
            pk_cache: RefCell::default(),
        }
    }
}

impl SecretManager {
    pub fn keys_for_poll(&self, poll_id: &PollId) -> Keypair<Ristretto> {
        let child = self.secret.digest(&poll_id.0);
        let keypair = Keypair::generate(&mut child.rng());
        self.pk_cache
            .borrow_mut()
            .insert(*poll_id, keypair.public().clone());
        keypair
    }

    pub fn public_key_for_poll(&self, poll_id: &PollId) -> PublicKey<Ristretto> {
        let pk = self.pk_cache.borrow().get(poll_id).cloned();
        pk.unwrap_or_else(|| self.keys_for_poll(poll_id).into_tuple().0)
    }
}
