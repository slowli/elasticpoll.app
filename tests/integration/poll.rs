//! Tests for polling logic.

use assert_matches::assert_matches;
use base64ct::{Base64UrlUnpadded, Encoding};
use elastic_elgamal::app::ChoiceVerificationError;
use rand_core::OsRng;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_test::*;

use elastic_elgamal_site::poll::{
    EncryptedVoteChoice, Keypair, ParticipantApplication, PollId, PollSpec, PollStage, PollState,
    PollType, TallierShare, TallierShareError, Vote, VoteChoice, VoteError,
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
    let value_str = json
        .pointer(pointer)
        .expect_throw("!!!")
        .as_str()
        .unwrap_throw();
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

#[wasm_bindgen_test]
fn poll_lifecycle_with_single_participant() {
    let poll_spec = single_choice_poll();
    let poll_id = PollId::for_spec(&poll_spec);

    let mut poll = PollState::new(poll_spec);
    assert_matches!(poll.stage(), PollStage::Participants { participants: 0 });

    let our_keys = Keypair::generate(&mut OsRng);
    let app = ParticipantApplication::new(&our_keys, &poll_id);
    app.validate(&poll_id).unwrap();

    poll.insert_participant(app);
    assert_matches!(poll.stage(), PollStage::Participants { participants: 1 });
    assert!(poll.has_participant(our_keys.public()));

    let other_app = ParticipantApplication::new(&our_keys, &poll_id);
    poll.insert_participant(other_app);
    assert_matches!(poll.stage(), PollStage::Participants { participants: 1 });

    poll.finalize_participants();
    assert_matches!(
        poll.stage(),
        PollStage::Voting {
            participants: 1,
            votes: 0,
        }
    );

    let our_choice = VoteChoice::SingleChoice(1);
    let vote = Vote::new(&our_keys, &poll_id, &poll, &our_choice);
    poll.insert_vote(&poll_id, vote).unwrap();
    assert_matches!(
        poll.stage(),
        PollStage::Voting {
            participants: 1,
            votes: 1,
        }
    );

    let new_choice = VoteChoice::SingleChoice(0);
    let new_vote = Vote::new(&our_keys, &poll_id, &poll, &new_choice);
    poll.insert_vote(&poll_id, new_vote).unwrap();
    assert_matches!(
        poll.stage(),
        PollStage::Voting {
            participants: 1,
            votes: 1,
        }
    );

    poll.finalize_votes();
    assert_matches!(
        poll.stage(),
        PollStage::Tallying {
            shares: 0,
            participants: 1,
        }
    );

    let our_share = TallierShare::new(&our_keys, &poll_id, &poll);
    poll.insert_tallier_share(&poll_id, our_share).unwrap();
    assert_matches!(poll.stage(), PollStage::Finished);

    let results = poll.results().unwrap();
    assert_eq!(results, [1, 0]);
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
    assert!(err.contains("challenge"), "{}", err);
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
    assert!(count > 20, "Too few valid mangled elements: {}", count);
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
    assert!(count > 20, "Too few valid mangled elements: {}", count);
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
