//! Poll data types.

use elastic_elgamal::{group::Ristretto, Ciphertext, DiscreteLogTable, PublicKey};
use js_sys::Date;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::UnwrapThrowExt;

use std::{error::Error as StdError, fmt, ops, str::FromStr};

use crate::utils::VecHelper;

mod managers;
mod participant;

pub use self::managers::{PollManager, SecretManager};
pub use self::participant::{
    Participant, ParticipantApplication, SubmittedTallierShare, SubmittedVote, TallierShare,
    TallierShareError, Vote, VoteChoice, VoteError,
};

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

// TODO: add specification
#[derive(Debug, Clone, Copy)]
pub enum PollStage {
    Participants { participants: usize },
    Voting { votes: usize, participants: usize },
    Tallying { shares: usize, participants: usize },
    Finished,
}

impl PollStage {
    pub const PARTICIPANTS_IDX: usize = 1;
    pub const VOTING_IDX: usize = 2;
    pub const TALLYING_IDX: usize = 3;
    pub const FINISHED_IDX: usize = 4;
    pub const MAX_INDEX: usize = Self::FINISHED_IDX;

    pub fn index(&self) -> usize {
        match self {
            Self::Participants { .. } => Self::PARTICIPANTS_IDX,
            Self::Voting { .. } => Self::VOTING_IDX,
            Self::Tallying { .. } => Self::TALLYING_IDX,
            Self::Finished => Self::FINISHED_IDX,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum TallyResult {
    InProgress,
    Finished(Vec<u64>),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tally_result: Option<TallyResult>,
}

impl PollState {
    fn new(spec: PollSpec) -> Self {
        Self {
            spec,
            created_at: Date::now(),
            participants: Vec::new(),
            shared_key: None,
            tally_result: None,
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
            match &self.tally_result {
                None => PollStage::Voting {
                    votes: self
                        .participants
                        .iter()
                        .filter(|p| p.vote.is_some())
                        .count(),
                    participants: self.participants.len(),
                },
                Some(TallyResult::InProgress) => PollStage::Tallying {
                    shares: self
                        .participants
                        .iter()
                        .filter(|p| p.tallier_share.is_some())
                        .count(),
                    participants: self.participants.len(),
                },
                Some(TallyResult::Finished(_)) => PollStage::Finished,
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
        assert!(
            self.shared_key.is_some(),
            "cannot insert a vote before participants are finalized"
        );
        assert!(
            self.tally_result.is_none(),
            "cannot insert a vote after votes are finalized"
        );

        let participant = self
            .participants
            .iter_mut()
            .find(|p| *p.public_key() == vote.public_key)
            .expect("vote does not come from an eligible voter");
        participant.vote = Some(vote.into());
    }

    pub fn finalize_votes(&mut self) {
        self.tally_result = Some(TallyResult::InProgress);
    }

    pub fn cumulative_choices(&self) -> Vec<Ciphertext<Ristretto>> {
        let mut ciphertexts = vec![Ciphertext::zero(); self.spec.options.len()];

        let participant_ciphertexts = self
            .participants
            .iter()
            .filter_map(|p| p.vote.as_ref().map(SubmittedVote::choices));
        for vote_ciphertexts in participant_ciphertexts {
            debug_assert_eq!(vote_ciphertexts.len(), ciphertexts.len());
            for (dest, src) in ciphertexts.iter_mut().zip(vote_ciphertexts) {
                *dest += *src;
            }
        }
        ciphertexts
    }

    pub fn insert_tallier_share(
        &mut self,
        poll_id: &PollId,
        share: TallierShare,
    ) -> Result<(), TallierShareError> {
        share.verify(poll_id, self)?;
        self.insert_unchecked_tallier_share(share);
        Ok(())
    }

    pub fn insert_unchecked_tallier_share(&mut self, share: TallierShare) {
        assert!(
            matches!(&self.tally_result, Some(TallyResult::InProgress)),
            "cannot insert tallier share when tallying is not active"
        );
        let participant = self
            .participants
            .iter_mut()
            .find(|p| *p.public_key() == share.public_key)
            .expect("vote does not come from an eligible voter");
        participant.tallier_share = Some(share.into());

        let all_shares_are_collected = self.participants.iter().all(|p| p.tallier_share.is_some());
        if all_shares_are_collected {
            let mut blinded_elements: Vec<_> = self
                .cumulative_choices()
                .into_iter()
                .map(|ciphertext| *ciphertext.blinded_element())
                .collect();
            for participant in &self.participants {
                let share = &participant.tallier_share.as_ref().unwrap_throw().inner;
                for (dest, src) in blinded_elements.iter_mut().zip(share.shares()) {
                    *dest -= src.as_element();
                }
            }

            let table = DiscreteLogTable::<Ristretto>::new(0..=self.participants.len() as u64);
            let decrypted_choices = blinded_elements
                .into_iter()
                .map(|elt| table.get(&elt).expect("cannot decrypt"))
                .collect();
            self.tally_result = Some(TallyResult::Finished(decrypted_choices));
        }
    }

    pub fn results(&self) -> Option<&[u64]> {
        if let Some(TallyResult::Finished(results)) = &self.tally_result {
            Some(results)
        } else {
            None
        }
    }
}
