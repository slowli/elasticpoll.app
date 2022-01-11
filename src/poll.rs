//! Poll data types.

use elastic_elgamal::{group::Ristretto, Keypair, ProofOfPossession, PublicKey};
use merlin::Transcript;
use rand_core::OsRng;
use secret_tree::SecretTree;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::UnwrapThrowExt;

use std::{
    cell::RefCell, collections::HashMap, error::Error as StdError, fmt, iter, ops, slice,
    str::FromStr,
};

use crate::utils::{local_storage, VecHelper};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollType {
    SingleChoice,
    MultiChoice,
}

impl FromStr for PollType {
    type Err = Box<dyn StdError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "single_choice" => Ok(Self::SingleChoice),
            "multi_choice" => Ok(Self::MultiChoice),
            _ => Err("Invalid `PollType` value".into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PollSpec {
    pub title: String,
    pub description: String,
    pub poll_type: PollType,
    pub nonce: u64,
    #[serde(with = "VecHelper::<String, 1, MAX_OPTIONS>")]
    pub options: Vec<String>,
}

/// Maximum allowed number of options in a poll (inclusive).
pub const MAX_OPTIONS: usize = 16;

/// Content-based poll ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollId([u8; 32]);

impl fmt::Display for PollId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD);
        formatter.write_str(&s)
    }
}

impl FromStr for PollId {
    type Err = Box<dyn StdError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const EXPECTED_INPUT_LEN: usize = 43; // ceil(32 * 4 / 3)

        if s.len() != EXPECTED_INPUT_LEN {
            return Err("Unexpected poll ID length".into());
        }
        let mut buffer = [0_u8; 32];
        let len = base64::decode_config_slice(s, base64::URL_SAFE_NO_PAD, &mut buffer)?;
        if len != 32 {
            return Err("Unexpected poll ID length".into());
        }
        Ok(Self(buffer))
    }
}

impl PollId {
    fn for_spec(spec: &PollSpec) -> Self {
        let json = serde_json::to_string(&spec).expect_throw("cannot serialize `PollSpec`");
        let id = Sha256::digest(json.as_str());
        let mut this = Self([0_u8; 32]);
        this.0.copy_from_slice(&id);
        this
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PollParticipant {
    pub public_key: PublicKey<Ristretto>,
    pub participation_consent: ProofOfPossession<Ristretto>,
}

impl PollParticipant {
    pub fn new(keypair: &Keypair<Ristretto>, poll_id: &PollId) -> Self {
        let mut transcript = Transcript::new(b"participation_consent");
        transcript.append_message(b"poll_id", &poll_id.0);
        let participation_consent =
            ProofOfPossession::new(slice::from_ref(keypair), &mut transcript, &mut OsRng);
        Self {
            public_key: keypair.public().clone(),
            participation_consent,
        }
    }

    pub fn validate(&self, poll_id: &PollId) -> Result<(), Box<dyn StdError>> {
        let mut transcript = Transcript::new(b"participation_consent");
        transcript.append_message(b"poll_id", &poll_id.0);
        self.participation_consent
            .verify(iter::once(&self.public_key), &mut transcript)
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PollStage {
    New,
    AddingParticipants { participants: usize },
}

impl PollStage {
    pub const MAX_INDEX: usize = 1;

    pub fn index(&self) -> usize {
        match self {
            Self::New => 0,
            Self::AddingParticipants { .. } => 1,
        }
    }
}

/// Ongoing or finished poll state.
#[derive(Debug, Serialize, Deserialize)]
pub struct PollState {
    pub spec: PollSpec,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<PollParticipant>,
}

impl PollState {
    fn new(spec: PollSpec) -> Self {
        Self {
            spec,
            participants: Vec::new(),
        }
    }

    pub fn stage(&self) -> PollStage {
        if self.participants.is_empty() {
            PollStage::New
        } else {
            PollStage::AddingParticipants {
                participants: self.participants.len(),
            }
        }
    }

    pub fn insert_participant(&mut self, participant: PollParticipant) {
        let existing_participant = self
            .participants
            .iter_mut()
            .find(|p| p.public_key == participant.public_key);
        if let Some(existing_participant) = existing_participant {
            *existing_participant = participant;
        } else {
            self.participants.push(participant);
        }
    }

    pub fn shared_public_key(&self) -> Option<PublicKey<Ristretto>> {
        self.participants
            .iter()
            .map(|participant| participant.public_key.clone())
            .reduce(ops::Add::add)
    }
}

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
    pub fn save_poll(&mut self, spec: PollSpec) -> PollId {
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
