//! Poll data types.

use elastic_elgamal::{
    app::{ChoiceParams, ChoiceVerificationError, EncryptedChoice, MultiChoice, SingleChoice},
    group::Ristretto,
    Keypair, ProofOfPossession, PublicKey, VerificationError,
};
use js_sys::Date;
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

impl PollType {
    fn as_human_string(self) -> &'static str {
        match self {
            Self::SingleChoice => "single choice",
            Self::MultiChoice => "multiple choice",
        }
    }
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
pub struct ParticipantApplication {
    pub public_key: PublicKey<Ristretto>,
    pub participation_consent: ProofOfPossession<Ristretto>,
}

impl ParticipantApplication {
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

/// Poll participant (voter / tallier).
#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    #[serde(flatten)]
    pub application: ParticipantApplication,
    pub created_at: f64,
    pub vote: Option<SubmittedVote>,
}

impl From<ParticipantApplication> for Participant {
    fn from(application: ParticipantApplication) -> Self {
        Self {
            application,
            created_at: Date::now(),
            vote: None,
        }
    }
}

impl Participant {
    pub fn public_key(&self) -> &PublicKey<Ristretto> {
        &self.application.public_key
    }
}

/// Plaintext voter's choice.
#[derive(Debug)]
pub enum VoteChoice {
    SingleChoice(usize),
    MultiChoice(Vec<bool>),
}

impl VoteChoice {
    pub fn default(spec: &PollSpec) -> Self {
        match spec.poll_type {
            PollType::SingleChoice => Self::SingleChoice(0),
            PollType::MultiChoice => Self::MultiChoice(vec![false; spec.options.len()]),
        }
    }

    pub fn is_selected(&self, option_idx: usize) -> bool {
        match self {
            Self::SingleChoice(choice) => *choice == option_idx,
            Self::MultiChoice(choices) => choices[option_idx],
        }
    }

    pub fn select(&mut self, option_idx: usize, select: bool) {
        match self {
            Self::SingleChoice(choice) => {
                if select {
                    *choice = option_idx;
                }
            }
            Self::MultiChoice(choices) => {
                choices[option_idx] = select;
            }
        }
    }

    fn poll_type(&self) -> PollType {
        match self {
            Self::SingleChoice(_) => PollType::SingleChoice,
            Self::MultiChoice(_) => PollType::MultiChoice,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EncryptedVoteChoice {
    SingleChoice(EncryptedChoice<Ristretto, SingleChoice>),
    MultiChoice(EncryptedChoice<Ristretto, MultiChoice>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vote {
    choice: EncryptedVoteChoice,
    public_key: PublicKey<Ristretto>,
    signature: ProofOfPossession<Ristretto>,
}

impl Vote {
    pub fn new(
        keypair: &Keypair<Ristretto>,
        poll_id: &PollId,
        poll: &PollState,
        choice: &VoteChoice,
    ) -> Self {
        debug_assert_eq!(poll.spec.poll_type, choice.poll_type());

        let shared_key = poll.finalized_shared_key().clone();
        let options_count = poll.spec.options.len();
        let choice = match choice {
            VoteChoice::SingleChoice(choice) => {
                let choice_params = ChoiceParams::single(shared_key, options_count);
                let enc = EncryptedChoice::single(&choice_params, *choice, &mut OsRng);
                EncryptedVoteChoice::SingleChoice(enc)
            }
            VoteChoice::MultiChoice(choices) => {
                let choice_params = ChoiceParams::multi(shared_key, options_count);
                let enc = EncryptedChoice::new(&choice_params, choices, &mut OsRng);
                EncryptedVoteChoice::MultiChoice(enc)
            }
        };
        Self::sign(keypair, poll_id, choice)
    }

    fn sign(keypair: &Keypair<Ristretto>, poll_id: &PollId, choice: EncryptedVoteChoice) -> Self {
        let mut transcript = Self::create_transcript(poll_id, &choice);
        let signature =
            ProofOfPossession::new(slice::from_ref(keypair), &mut transcript, &mut OsRng);

        Self {
            choice,
            public_key: keypair.public().clone(),
            signature,
        }
    }

    // Serializing to JSON is quite fragile, but should work (`VoteChoice` doesn't contain
    // any related non-determinism, such as `HashMap`s).
    fn create_transcript(poll_id: &PollId, choice: &EncryptedVoteChoice) -> Transcript {
        let serialized_choice =
            serde_json::to_string(choice).expect_throw("cannot serialize `VoteChoice`");
        let mut transcript = Transcript::new(b"vote");
        transcript.append_message(b"poll_id", &poll_id.0);
        transcript.append_message(b"choice", serialized_choice.as_bytes());
        transcript
    }

    fn verify(&self, poll_id: &PollId, poll: &PollState) -> Result<(), VoteError> {
        // Check that the voter is eligible.
        if !poll
            .participants
            .iter()
            .any(|p| *p.public_key() == self.public_key)
        {
            return Err(VoteError::IneligibleVoter);
        }

        // Check signature.
        let mut transcript = Self::create_transcript(poll_id, &self.choice);
        self.signature
            .verify(iter::once(&self.public_key), &mut transcript)
            .map_err(VoteError::Signature)?;

        // Check choice.
        let shared_key = poll.finalized_shared_key().clone();
        match &self.choice {
            EncryptedVoteChoice::SingleChoice(choice) => {
                VoteError::ensure_choice_type(poll.spec.poll_type, PollType::SingleChoice)?;
                let choice_params = ChoiceParams::single(shared_key, poll.spec.options.len());
                choice.verify(&choice_params).map_err(VoteError::Choice)?;
            }
            EncryptedVoteChoice::MultiChoice(choice) => {
                VoteError::ensure_choice_type(poll.spec.poll_type, PollType::MultiChoice)?;
                let choice_params = ChoiceParams::multi(shared_key, poll.spec.options.len());
                choice.verify(&choice_params).map_err(VoteError::Choice)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum VoteError {
    IneligibleVoter,
    ChoiceType {
        expected: PollType,
        actual: PollType,
    },
    Signature(VerificationError),
    Choice(ChoiceVerificationError),
}

impl fmt::Display for VoteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IneligibleVoter => formatter.write_str("voter is not eligible"),
            Self::ChoiceType { expected, actual } => {
                write!(
                    formatter,
                    "unexpected type of submitted choice: expected {}, got {}",
                    expected.as_human_string(),
                    actual.as_human_string()
                )
            }
            Self::Signature(err) => write!(formatter, "cannot verify voter's signature: {}", err),
            Self::Choice(err) => write!(formatter, "cannot verify choice: {}", err),
        }
    }
}

impl VoteError {
    fn ensure_choice_type(expected: PollType, actual: PollType) -> Result<(), Self> {
        if expected == actual {
            Ok(())
        } else {
            Err(Self::ChoiceType { expected, actual })
        }
    }
}

impl StdError for VoteError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::IneligibleVoter | Self::ChoiceType { .. } => None,
            Self::Signature(err) => Some(err),
            Self::Choice(err) => Some(err),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmittedVote {
    #[serde(flatten)]
    pub inner: Vote,
    /// Vote hash (to sync votes among participants).
    pub hash: String,
    /// Unix timestamp (in milliseconds).
    pub submitted_at: f64,
}

impl From<Vote> for SubmittedVote {
    fn from(vote: Vote) -> Self {
        let json = serde_json::to_string(&vote.choice)
            .expect_throw("cannot serialize `EncryptedVoteChoice`");
        let vote_hash = Sha256::digest(&json);

        Self {
            inner: vote,
            hash: base64::encode_config(&vote_hash, base64::URL_SAFE_NO_PAD),
            submitted_at: Date::now(),
        }
    }
}

// TODO: add specification
#[derive(Debug, Clone, Copy)]
pub enum PollStage {
    Participants { participants: usize },
    Voting { votes: usize, participants: usize },
}

impl PollStage {
    pub const PARTICIPANTS_IDX: usize = 1;
    pub const VOTING_IDX: usize = 2;
    pub const MAX_INDEX: usize = 3;

    pub fn index(&self) -> usize {
        match self {
            Self::Participants { .. } => Self::PARTICIPANTS_IDX,
            Self::Voting { .. } => Self::VOTING_IDX,
        }
    }
}

/// Ongoing or finished poll state.
#[derive(Debug, Serialize, Deserialize)]
pub struct PollState {
    /// Unix timestamp (in milliseconds).
    pub created_at: f64,
    spec: PollSpec,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    participants: Vec<Participant>,
    /// Shared encryption key for the voting. Only present if the set of participants is final.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shared_key: Option<PublicKey<Ristretto>>,
}

impl PollState {
    fn new(spec: PollSpec) -> Self {
        Self {
            spec,
            created_at: Date::now(),
            participants: Vec::new(),
            shared_key: None,
        }
    }

    pub fn spec(&self) -> &PollSpec {
        &self.spec
    }

    pub fn stage(&self) -> PollStage {
        if self.shared_key.is_none() {
            PollStage::Participants {
                participants: self.participants.len(),
            }
        } else {
            PollStage::Voting {
                votes: self
                    .participants
                    .iter()
                    .filter(|p| p.vote.is_some())
                    .count(),
                participants: self.participants.len(),
            }
        }
    }

    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }

    pub fn insert_participant(&mut self, application: ParticipantApplication) {
        assert!(
            self.shared_key.is_none(),
            "cannot change participants once they are finalized"
        );

        let existing_participant = self
            .participants
            .iter_mut()
            .find(|p| *p.public_key() == application.public_key);
        if let Some(existing_participant) = existing_participant {
            *existing_participant = application.into();
        } else {
            self.participants.push(application.into());
        }
    }

    pub fn remove_participant(&mut self, index: usize) {
        assert!(
            self.shared_key.is_none(),
            "cannot change participants once they are finalized"
        );
        self.participants.remove(index);
    }

    pub fn shared_key(&self) -> Option<PublicKey<Ristretto>> {
        self.participants
            .iter()
            .map(|participant| participant.public_key().clone())
            .reduce(ops::Add::add)
    }

    fn finalized_shared_key(&self) -> &PublicKey<Ristretto> {
        self.shared_key
            .as_ref()
            .expect_throw("set of participants is not finalized")
    }

    pub fn finalize_participants(&mut self) {
        self.shared_key = self.shared_key();
    }

    pub fn contains_votes(&self) -> bool {
        self.participants
            .iter()
            .any(|participant| participant.vote.is_some())
    }

    pub fn insert_vote(&mut self, poll_id: &PollId, vote: Vote) -> Result<(), VoteError> {
        vote.verify(poll_id, self)?;
        self.insert_unchecked_vote(vote);
        Ok(())
    }

    pub fn insert_unchecked_vote(&mut self, vote: Vote) {
        let participant = self
            .participants
            .iter_mut()
            .find(|p| *p.public_key() == vote.public_key)
            .expect("vote does not come from an eligible voter");
        participant.vote = Some(vote.into());
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
            secret: SecretTree::from_slice(&[11; 32]).unwrap(),
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
