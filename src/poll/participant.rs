//! [`PollParticipant`] and tightly related types.

use base64ct::{Base64UrlUnpadded, Encoding};
use elastic_elgamal::{
    app::{ChoiceParams, ChoiceVerificationError, EncryptedChoice, MultiChoice, SingleChoice},
    CandidateDecryption, Ciphertext, LogEqualityProof, ProofOfPossession, VerifiableDecryption,
    VerificationError,
};
use js_sys::Date;
use merlin::Transcript;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::UnwrapThrowExt;

use std::{convert::TryFrom, error::Error as StdError, fmt, iter, slice};

use super::{Group, Keypair, PollId, PollSpec, PollState, PollType, PublicKey, PublicKeyBytes};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantApplication {
    pub public_key: PublicKey,
    pub participation_consent: ProofOfPossession<Group>,
}

impl ParticipantApplication {
    pub fn new(keypair: &Keypair, poll_id: &PollId) -> Self {
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
    pub tallier_share: Option<SubmittedTallierShare>,
}

impl From<ParticipantApplication> for Participant {
    fn from(application: ParticipantApplication) -> Self {
        Self {
            application,
            created_at: Date::now(),
            vote: None,
            tallier_share: None,
        }
    }
}

impl Participant {
    pub fn public_key(&self) -> &PublicKey {
        &self.application.public_key
    }

    pub fn public_key_bytes(&self) -> PublicKeyBytes {
        PublicKeyBytes::try_from(self.public_key().as_bytes())
            .expect_throw("unexpected public key byte size")
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EncryptedVoteChoice {
    SingleChoice(EncryptedChoice<Group, SingleChoice>),
    MultiChoice(EncryptedChoice<Group, MultiChoice>),
}

impl EncryptedVoteChoice {
    fn choices_unchecked(&self) -> &[Ciphertext<Group>] {
        match self {
            Self::SingleChoice(choice) => choice.choices_unchecked(),
            Self::MultiChoice(choice) => choice.choices_unchecked(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    choice: EncryptedVoteChoice,
    pub(super) public_key: PublicKey,
    signature: ProofOfPossession<Group>,
}

impl Vote {
    pub fn new(keypair: &Keypair, poll_id: &PollId, poll: &PollState, choice: &VoteChoice) -> Self {
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

    // Public for testing
    pub fn sign(keypair: &Keypair, poll_id: &PollId, choice: EncryptedVoteChoice) -> Self {
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

    pub(super) fn verify(&self, poll_id: &PollId, poll: &PollState) -> Result<(), VoteError> {
        // Check that the voter is eligible.
        if !poll.has_participant(&self.public_key) {
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
            Self::Signature(err) => write!(formatter, "cannot verify voter's signature: {err}"),
            Self::Choice(err) => write!(formatter, "cannot verify choice: {err}"),
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
        let vote_hash = Sha256::digest(json);

        Self {
            inner: vote,
            hash: Base64UrlUnpadded::encode_string(&vote_hash),
            submitted_at: Date::now(),
        }
    }
}

impl SubmittedVote {
    pub(super) fn choices(&self) -> &[Ciphertext<Group>] {
        self.inner.choice.choices_unchecked()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallierShare {
    shares: Vec<ShareWithProof>,
    pub(super) public_key: PublicKey,
}

impl TallierShare {
    pub fn new(keypair: &Keypair, poll_id: &PollId, poll_state: &PollState) -> Self {
        let transcript = Self::create_transcript(poll_id, poll_state);
        let ciphertexts = poll_state.cumulative_choices();
        let shares = ciphertexts.into_iter().map(|ciphertext| {
            let (share, proof) =
                VerifiableDecryption::new(ciphertext, keypair, &mut transcript.clone(), &mut OsRng);
            ShareWithProof {
                share: share.into(),
                proof,
            }
        });

        Self {
            shares: shares.collect(),
            public_key: keypair.public().clone(),
        }
    }

    fn create_transcript(poll_id: &PollId, poll_state: &PollState) -> Transcript {
        let mut transcript = Transcript::new(b"tallier_share");
        transcript.append_message(b"poll_id", &poll_id.0);
        // Commit to the shared key and number of participants.
        transcript.append_message(b"shared_key", poll_state.finalized_shared_key().as_bytes());
        transcript.append_u64(b"n", poll_state.participants.len() as u64);
        transcript
    }

    pub(super) fn shares(&self) -> impl Iterator<Item = VerifiableDecryption<Group>> + '_ {
        self.shares
            .iter()
            .map(|share_with_proof| share_with_proof.share.into_unchecked())
    }

    pub(super) fn verify(
        &self,
        poll_id: &PollId,
        poll: &PollState,
    ) -> Result<(), TallierShareError> {
        // Check that all shares were submitted.
        TallierShareError::ensure_options_count(poll.spec.options.len(), self.shares.len())?;
        // Check that the voter is eligible.
        if !poll.has_participant(&self.public_key) {
            return Err(TallierShareError::IneligibleTallier);
        }

        let transcript = Self::create_transcript(poll_id, poll);
        let ciphertexts = poll.cumulative_choices();

        let it = self.shares.iter().enumerate().zip(ciphertexts);
        for ((i, share_with_proof), ciphertext) in it {
            share_with_proof
                .share
                .verify(
                    ciphertext,
                    &self.public_key,
                    &share_with_proof.proof,
                    &mut transcript.clone(), // transcripts for all proofs are independent
                )
                .map_err(|err| TallierShareError::InvalidShare { index: i, err })?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum TallierShareError {
    OptionsCount {
        expected: usize,
        actual: usize,
    },
    IneligibleTallier,
    InvalidShare {
        index: usize,
        err: VerificationError,
    },
}

impl fmt::Display for TallierShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OptionsCount { expected, actual } => {
                write!(
                    formatter,
                    "unexpected number of options: expected {expected}, got {actual}"
                )
            }
            Self::IneligibleTallier => formatter.write_str("tallier is not eligible"),
            Self::InvalidShare { index, err } => {
                write!(
                    formatter,
                    "cannot verify share for option #{}: {err}",
                    *index + 1
                )
            }
        }
    }
}

impl StdError for TallierShareError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidShare { err, .. } => Some(err),
            _ => None,
        }
    }
}

impl TallierShareError {
    fn ensure_options_count(expected: usize, actual: usize) -> Result<(), Self> {
        if expected == actual {
            Ok(())
        } else {
            Err(Self::OptionsCount { expected, actual })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShareWithProof {
    pub(super) share: CandidateDecryption<Group>,
    proof: LogEqualityProof<Group>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmittedTallierShare {
    #[serde(flatten)]
    pub inner: TallierShare,
    /// Unix timestamp (in milliseconds).
    pub submitted_at: f64,
}

impl From<TallierShare> for SubmittedTallierShare {
    fn from(share: TallierShare) -> Self {
        Self {
            inner: share,
            submitted_at: Date::now(),
        }
    }
}
