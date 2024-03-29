//! Tests for polling logic.

use assert_matches::assert_matches;
use base64ct::{Base64UrlUnpadded, Encoding};
use elastic_elgamal::app::ChoiceVerificationError;
use rand::{rngs::OsRng, Rng};
use serde::Serialize;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_test::*;

use std::fmt;

use elasticpoll_wasm::poll::{
    EncryptedVoteChoice, Keypair, ParticipantApplication, PollId, PollSpec, PollStage, PollState,
    PollType, SubmittedTallierShare, SubmittedVote, TallierShare, TallierShareError, Vote,
    VoteChoice, VoteError,
};

fn single_choice_poll() -> PollSpec {
    PollSpec {
        title: "Sample poll".to_owned(),
        description: "".to_owned(),
        poll_type: PollType::SingleChoice,
        nonce: 0,
        options: vec!["Option #1".to_owned(), "Option #2".to_owned()],
    }
}

fn mangle_bytes(
    json: serde_json::Value,
    pointer: &'static str,
    bits_to_mangle: impl Iterator<Item = usize>,
) -> impl Iterator<Item = serde_json::Value> {
    let value_str = json.pointer(pointer).unwrap_throw().as_str().unwrap_throw();
    let mut value = [0_u8; 32];
    Base64UrlUnpadded::decode(value_str, &mut value).unwrap_throw();

    bits_to_mangle.map(move |bit_idx| {
        let mut json = json.clone();
        let mut mangled_value = value;
        mangled_value[bit_idx / 8] ^= 1 << (bit_idx % 8);
        let mangled_value = Base64UrlUnpadded::encode_string(&mangled_value);
        *json.pointer_mut(pointer).unwrap_throw() = mangled_value.into();
        json
    })
}

fn mangle_group_element(
    json: serde_json::Value,
    pointer: &'static str,
) -> impl Iterator<Item = serde_json::Value> {
    mangle_bytes(json, pointer, 0..248)
}

fn mangle_scalar(
    json: serde_json::Value,
    pointer: &'static str,
) -> impl Iterator<Item = serde_json::Value> {
    mangle_bytes(json, pointer, 0..252)
}

#[wasm_bindgen_test]
fn mangle_group_element_works_as_expected() {
    let test_value = "9GrwVAQ10kkX80-0SSpdPMyJTFpvV4GGCWzCiHutjXQ";
    let json = serde_json::json!({
        "test": test_value,
    });
    for mangled_json in mangle_group_element(json, "/test") {
        let mangled_value = mangled_json
            .pointer("/test")
            .unwrap_throw()
            .as_str()
            .unwrap_throw();

        assert_eq!(mangled_value.len(), test_value.len());
        assert_ne!(mangled_value, test_value);
        let differing_chars = mangled_value
            .chars()
            .zip(test_value.chars())
            .filter(|(mangled_c, c)| c != mangled_c)
            .count();
        assert_eq!(differing_chars, 1);
    }
}

/// Marker trait for values with a `submitted_at` timestamp.
trait WithTimestamp: Serialize + fmt::Debug {}

impl WithTimestamp for SubmittedVote {}
impl WithTimestamp for SubmittedTallierShare {}

fn assert_eq_ignoring_timestamps<T: WithTimestamp>(lhs: Option<&T>, rhs: Option<&T>) {
    match (lhs, rhs) {
        (None, None) => { /* ok */ }
        (Some(lhs), Some(rhs)) => {
            let mut lhs_value = serde_json::to_value(lhs).unwrap_throw();
            lhs_value
                .as_object_mut()
                .unwrap_throw()
                .remove("submitted_at");
            let mut rhs_value = serde_json::to_value(rhs).unwrap_throw();
            rhs_value
                .as_object_mut()
                .unwrap_throw()
                .remove("submitted_at");
            assert_eq!(lhs_value, rhs_value);
        }
        _ => panic!("{lhs:?} != {rhs:?}"),
    }
}

fn assert_poll_export(poll: &PollState) {
    let exported = poll.export();
    let (_, imported) = PollState::import(exported).unwrap_throw();
    assert_eq!(imported.stage(), poll.stage());

    let it = poll.participants().iter().zip(imported.participants());
    for (participant, imported_participant) in it {
        assert_eq!(participant.public_key(), imported_participant.public_key());
        assert_eq_ignoring_timestamps(
            participant.vote.as_ref(),
            imported_participant.vote.as_ref(),
        );
        assert_eq_ignoring_timestamps(
            participant.tallier_share.as_ref(),
            imported_participant.tallier_share.as_ref(),
        );
    }
}

fn test_poll_lifecycle(participant_count: usize) {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);
    let keys: Vec<_> = (0..participant_count)
        .map(|_| Keypair::generate(&mut OsRng))
        .collect();

    for our_keys in &keys {
        let app = ParticipantApplication::new(our_keys, &poll_id);
        app.validate(&poll_id).unwrap();
        poll.insert_participant(app);
    }
    assert_eq!(
        poll.stage(),
        PollStage::Participants {
            participants: participant_count
        }
    );
    assert_poll_export(&poll);

    poll.finalize_participants();
    assert_eq!(
        poll.stage(),
        PollStage::Voting {
            participants: participant_count,
            votes: 0,
        }
    );

    let mut expected_results = vec![0_u64; poll.spec().options.len()];
    for (i, our_keys) in keys.iter().enumerate() {
        let our_choice = OsRng.gen_range(0..expected_results.len());
        expected_results[our_choice] += 1;
        let our_choice = VoteChoice::SingleChoice(our_choice);
        let vote = Vote::new(our_keys, &poll_id, &poll, &our_choice);
        poll.insert_vote(&poll_id, vote).unwrap();

        assert_eq!(
            poll.stage(),
            PollStage::Voting {
                participants: participant_count,
                votes: i + 1,
            }
        );
        assert_poll_export(&poll);
    }

    poll.finalize_votes();
    assert_eq!(
        poll.stage(),
        PollStage::Tallying {
            shares: 0,
            participants: participant_count,
        }
    );

    for (i, our_keys) in keys.iter().enumerate() {
        let our_share = TallierShare::new(our_keys, &poll_id, &poll);
        poll.insert_tallier_share(&poll_id, our_share).unwrap();

        if i + 1 < keys.len() {
            assert_eq!(
                poll.stage(),
                PollStage::Tallying {
                    shares: i + 1,
                    participants: participant_count,
                }
            );
        } else {
            assert_eq!(poll.stage(), PollStage::Finished);
        }
        assert_poll_export(&poll);
    }

    let results = poll.results().unwrap();
    assert_eq!(results, &expected_results);
}

#[wasm_bindgen_test]
fn poll_lifecycle_with_single_participant() {
    test_poll_lifecycle(1);
}

#[wasm_bindgen_test]
fn poll_lifecycle_with_2_participants() {
    test_poll_lifecycle(2);
}

#[wasm_bindgen_test]
fn poll_lifecycle_with_3_participants() {
    test_poll_lifecycle(3);
}

#[wasm_bindgen_test]
fn poll_lifecycle_with_5_participants() {
    test_poll_lifecycle(5);
}

#[wasm_bindgen_test]
fn invalid_poll_id_in_participant_application() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let our_keys = Keypair::generate(&mut OsRng);
    let bogus_poll_id: PollId = "9GrwVAQ10kkX80-0SSpdPMyJTFpvV4GGCWzCiHutjXQ"
        .parse()
        .unwrap();
    let app = ParticipantApplication::new(&our_keys, &bogus_poll_id);

    let err = app.validate(&poll_id).unwrap_err();
    let err = err.to_string();
    assert!(err.contains("challenge"), "{err}");
}

#[wasm_bindgen_test]
fn participant_application_with_mangled_public_key() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let our_keys = Keypair::generate(&mut OsRng);

    let app = ParticipantApplication::new(&our_keys, &poll_id);
    let app_json = serde_json::to_value(app).unwrap_throw();
    let mut count = 0;
    for mangled_app_json in mangle_group_element(app_json, "/public_key") {
        let mangled_app: ParticipantApplication = match serde_json::from_value(mangled_app_json) {
            Ok(app) => app,
            Err(_) => continue, // can happen since not all mangled group elements are valid
        };
        count += 1;
        mangled_app.validate(&poll_id).unwrap_err();
    }
    assert!(count > 20, "Too few valid mangled elements: {count}");
}

#[wasm_bindgen_test]
fn participant_application_with_mangled_proof() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    let app_json = serde_json::to_value(app).unwrap_throw();

    for mangled_app_json in mangle_scalar(app_json.clone(), "/participation_consent/challenge") {
        let mangled_app: ParticipantApplication =
            serde_json::from_value(mangled_app_json).unwrap_throw();
        mangled_app.validate(&poll_id).unwrap_err();
    }

    for mangled_app_json in mangle_scalar(app_json, "/participation_consent/responses/0") {
        let mangled_app: ParticipantApplication =
            serde_json::from_value(mangled_app_json).unwrap_throw();
        mangled_app.validate(&poll_id).unwrap_err();
    }
}

#[wasm_bindgen_test]
fn vote_from_ineligible_voter() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(app);
    poll.finalize_participants();

    let other_keys = Keypair::generate(&mut OsRng);
    assert_ne!(our_keys.public(), other_keys.public());
    let vote = Vote::new(&other_keys, &poll_id, &poll, &VoteChoice::SingleChoice(1));

    let err = poll.insert_vote(&poll_id, vote).unwrap_err();
    assert_matches!(err, VoteError::IneligibleVoter);
}

fn extract_choice_json(vote: Vote) -> serde_json::Value {
    serde_json::to_value(vote)
        .unwrap_throw()
        .as_object_mut()
        .unwrap_throw()
        .remove("choice")
        .unwrap_throw()
}

#[wasm_bindgen_test]
fn vote_with_invalid_choice_type() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(app);
    poll.finalize_participants();

    let vote = Vote::new(&our_keys, &poll_id, &poll, &VoteChoice::SingleChoice(1));
    let mut choice_json = extract_choice_json(vote);
    *choice_json.pointer_mut("/type").unwrap_throw() = String::from("multi_choice").into();
    *choice_json.pointer_mut("/sum_proof").unwrap_throw() = serde_json::Value::Null;
    let mangled_choice: EncryptedVoteChoice = serde_json::from_value(choice_json).unwrap_throw();
    let mangled_vote = Vote::sign(&our_keys, &poll_id, mangled_choice);

    let err = poll.insert_vote(&poll_id, mangled_vote).unwrap_err();
    assert_matches!(
        err,
        VoteError::ChoiceType {
            expected: PollType::SingleChoice,
            actual: PollType::MultiChoice,
        }
    );
}

#[wasm_bindgen_test]
fn vote_with_invalid_signature() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(app);
    poll.finalize_participants();

    let vote = Vote::new(&our_keys, &poll_id, &poll, &VoteChoice::SingleChoice(1));
    let vote_json = serde_json::to_value(vote).unwrap_throw();

    for mangled_vote_json in mangle_scalar(vote_json.clone(), "/signature/challenge") {
        let mangled_vote: Vote = serde_json::from_value(mangled_vote_json).unwrap_throw();
        let err = poll.insert_vote(&poll_id, mangled_vote).unwrap_err();
        assert_matches!(err, VoteError::Signature(_));
    }
    for mangled_vote_json in mangle_scalar(vote_json.clone(), "/signature/responses/0") {
        let mangled_vote: Vote = serde_json::from_value(mangled_vote_json).unwrap_throw();
        let err = poll.insert_vote(&poll_id, mangled_vote).unwrap_err();
        assert_matches!(err, VoteError::Signature(_));
    }

    // Mangling `choice` should invalidate the signature as well.
    let votes_with_mangled_range_proof =
        mangle_scalar(vote_json.clone(), "/choice/range_proof/common_challenge");
    for mangled_vote_json in votes_with_mangled_range_proof {
        let mangled_vote: Vote = serde_json::from_value(mangled_vote_json).unwrap_throw();
        let err = poll.insert_vote(&poll_id, mangled_vote).unwrap_err();
        assert_matches!(err, VoteError::Signature(_));
    }

    let votes_with_mangled_sum_proof = mangle_scalar(vote_json, "/choice/sum_proof/challenge");
    for mangled_vote_json in votes_with_mangled_sum_proof {
        let mangled_vote: Vote = serde_json::from_value(mangled_vote_json).unwrap_throw();
        let err = poll.insert_vote(&poll_id, mangled_vote).unwrap_err();
        assert_matches!(err, VoteError::Signature(_));
    }
}

#[wasm_bindgen_test]
fn vote_with_invalid_proofs() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(app);
    poll.finalize_participants();

    let vote = Vote::new(&our_keys, &poll_id, &poll, &VoteChoice::SingleChoice(1));
    let choice_json = extract_choice_json(vote);

    let choices_with_mangled_range_proof =
        mangle_scalar(choice_json.clone(), "/range_proof/common_challenge");
    for mangled_choice_json in choices_with_mangled_range_proof {
        let mangled_choice: EncryptedVoteChoice =
            serde_json::from_value(mangled_choice_json).unwrap_throw();
        let vote = Vote::sign(&our_keys, &poll_id, mangled_choice);
        let err = poll.insert_vote(&poll_id, vote).unwrap_err();
        assert_matches!(err, VoteError::Choice(ChoiceVerificationError::Range(_)));
    }

    let choices_with_mangled_sum_proof = mangle_scalar(choice_json, "/sum_proof/challenge");
    for mangled_choice_json in choices_with_mangled_sum_proof {
        let mangled_choice: EncryptedVoteChoice =
            serde_json::from_value(mangled_choice_json).unwrap_throw();
        let vote = Vote::sign(&our_keys, &poll_id, mangled_choice);
        let err = poll.insert_vote(&poll_id, vote).unwrap_err();
        assert_matches!(err, VoteError::Choice(ChoiceVerificationError::Sum(_)));
    }
}

fn prepare_poll_for_tallying() -> (PollId, PollState, Keypair) {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);
    let mut poll = PollState::new(poll_spec);

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(app);
    poll.finalize_participants();
    let vote = Vote::new(&our_keys, &poll_id, &poll, &VoteChoice::SingleChoice(1));
    poll.insert_vote(&poll_id, vote).unwrap_throw();
    poll.finalize_votes();

    (poll_id, poll, our_keys)
}

#[wasm_bindgen_test]
fn tallier_share_from_ineligible_tallier() {
    let (poll_id, mut poll, our_keys) = prepare_poll_for_tallying();
    let other_keys = Keypair::generate(&mut OsRng);
    assert_ne!(our_keys.public(), other_keys.public());
    let share = TallierShare::new(&other_keys, &poll_id, &poll);

    let err = poll.insert_tallier_share(&poll_id, share).unwrap_err();
    assert_matches!(err, TallierShareError::IneligibleTallier);
}

#[wasm_bindgen_test]
fn tallier_share_with_invalid_dh_element() {
    let (poll_id, mut poll, our_keys) = prepare_poll_for_tallying();
    let share = TallierShare::new(&our_keys, &poll_id, &poll);
    let share_json = serde_json::to_value(share).unwrap_throw();

    let mut count = 0;
    let mangled_jsons = mangle_group_element(share_json, "/shares/0/share/dh_element");
    for mangled_share_json in mangled_jsons {
        let mangled_share: TallierShare = match serde_json::from_value(mangled_share_json) {
            Ok(share) => share,
            Err(_) => continue,
        };
        count += 1;

        let err = poll
            .insert_tallier_share(&poll_id, mangled_share)
            .unwrap_err();
        assert_matches!(err, TallierShareError::InvalidShare { index: 0, .. });
    }
    assert!(count > 20, "Too few valid mangled elements: {count}");
}

#[wasm_bindgen_test]
fn tallier_share_with_invalid_proof() {
    let (poll_id, mut poll, our_keys) = prepare_poll_for_tallying();
    let share = TallierShare::new(&our_keys, &poll_id, &poll);
    let share_json = serde_json::to_value(share).unwrap_throw();

    for mangled_share_json in mangle_scalar(share_json.clone(), "/shares/1/proof/challenge") {
        let mangled_share: TallierShare = serde_json::from_value(mangled_share_json).unwrap_throw();
        let err = poll
            .insert_tallier_share(&poll_id, mangled_share)
            .unwrap_err();
        assert_matches!(err, TallierShareError::InvalidShare { index: 1, .. });
    }
    for mangled_share_json in mangle_scalar(share_json, "/shares/0/proof/response") {
        let mangled_share: TallierShare = serde_json::from_value(mangled_share_json).unwrap_throw();
        let err = poll
            .insert_tallier_share(&poll_id, mangled_share)
            .unwrap_err();
        assert_matches!(err, TallierShareError::InvalidShare { index: 0, .. });
    }
}
