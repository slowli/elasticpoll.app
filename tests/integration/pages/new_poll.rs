//! Tests for the new poll wizard.

use assert_matches::assert_matches;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_test::*;
use web_sys::{Element, HtmlButtonElement, HtmlInputElement, HtmlTextAreaElement};

use super::{assert_no_child, extract_feedback, select_elements, select_single_element, TestRig};
use elastic_elgamal_site::{
    js::ExportedDataType,
    pages::{NewPoll, NewPollMessage, NewPollProperties},
    poll::PollSpec,
};

fn input_col(root: &Element, input_selector: &str) -> Element {
    let input = select_single_element(root, input_selector);
    input.parent_element().expect_throw("input has no parent")
}

#[wasm_bindgen_test]
fn error_on_empty_title() {
    let rig = TestRig::<NewPoll>::new(NewPollProperties::default());
    let title_input_parent = input_col(&rig.root_element, "#title");
    assert_no_child(&title_input_parent, ".invalid-feedback");

    rig.send_message(NewPollMessage::TitleSet("New poll".to_owned()));
    let title_input_parent = input_col(&rig.root_element, "#title");
    assert_no_child(&title_input_parent, ".invalid-feedback");

    rig.send_message(NewPollMessage::TitleSet(String::new()));
    let title_input_parent = input_col(&rig.root_element, "#title");
    let feedback = extract_feedback(&title_input_parent);
    assert!(feedback.contains("cannot be empty"), "{}", feedback);
}

#[wasm_bindgen_test]
fn error_on_duplicate_option() {
    let rig = TestRig::<NewPoll>::new(NewPollProperties::default());
    assert_no_child(&rig.root_element, ".invalid-feedback");

    rig.send_message(NewPollMessage::OptionSet(0, "Option #2".to_owned()));
    assert_no_child(&rig.root_element, ".invalid-feedback");

    rig.send_message(NewPollMessage::OptionAdded);
    for option_id in ["#option-0", "#option-1"] {
        let option_col = input_col(&rig.root_element, option_id)
            .parent_element()
            .unwrap_throw();
        let feedback = extract_feedback(&option_col);
        assert!(
            feedback.contains("descriptions must be unique"),
            "{}",
            feedback
        );
    }

    rig.send_message(NewPollMessage::OptionRemoved(0));
    assert_no_child(&rig.root_element, ".invalid-feedback");
}

fn assert_option_controls(option_col: &Element, up_disabled: bool, down_disabled: bool) {
    let buttons: Vec<_> = select_elements(option_col, "button")
        .map(|btn| btn.dyn_into::<HtmlButtonElement>().unwrap_throw())
        .collect();
    assert_eq!(buttons.len(), 3);

    assert!(buttons.iter().any(|btn| btn.title().contains("Remove")));

    let move_up_btn = buttons
        .iter()
        .find(|btn| btn.title().contains("upper"))
        .unwrap_throw();
    assert_eq!(move_up_btn.disabled(), up_disabled);

    let move_down_btn = buttons
        .iter()
        .find(|btn| btn.title().contains("lower"))
        .unwrap_throw();
    assert_eq!(move_down_btn.disabled(), down_disabled);
}

#[wasm_bindgen_test]
fn option_controls() {
    let rig = TestRig::<NewPoll>::new(NewPollProperties::default());
    let option_col = input_col(&rig.root_element, "#option-0");
    assert_no_child(&option_col, "button");

    rig.send_message(NewPollMessage::OptionAdded);
    let option_col = input_col(&rig.root_element, "#option-0");
    assert_option_controls(&option_col, true, false);
    let option_col = input_col(&rig.root_element, "#option-1");
    assert_option_controls(&option_col, false, true);

    rig.send_message(NewPollMessage::OptionAdded);
    let option_col = input_col(&rig.root_element, "#option-0");
    assert_option_controls(&option_col, true, false);
    let option_col = input_col(&rig.root_element, "#option-1");
    assert_option_controls(&option_col, false, false);
    let option_col = input_col(&rig.root_element, "#option-2");
    assert_option_controls(&option_col, false, true);

    rig.send_message(NewPollMessage::OptionRemoved(0));
    rig.send_message(NewPollMessage::OptionRemoved(1));
    let option_col = input_col(&rig.root_element, "#option-0");
    assert_no_child(&option_col, "button");
}

fn extract_spec(rig: &TestRig<NewPoll>) -> PollSpec {
    let spec_json = select_single_element(&rig.root_element, "#poll-spec")
        .dyn_into::<HtmlTextAreaElement>()
        .unwrap_throw()
        .value();
    serde_json::from_str::<PollSpec>(&spec_json).unwrap_throw()
}

#[wasm_bindgen_test]
fn exporting_a_poll() {
    let rig = TestRig::<NewPoll>::new(NewPollProperties::default());
    rig.send_message(NewPollMessage::ExportRequested);

    let export = rig.export_calls().assert_called_once();
    assert_matches!(export.ty, ExportedDataType::PollSpec);
    let spec: PollSpec = serde_json::from_str(&export.data).unwrap_throw();
    assert_eq!(spec.options, ["Option #1"]);
}

#[wasm_bindgen_test]
fn importing_a_poll() {
    let rig = TestRig::<NewPoll>::new(NewPollProperties::default());
    extract_spec(&rig);
    let spec_col = input_col(&rig.root_element, "#poll-spec");
    assert_no_child(&spec_col, ".invalid-feedback");

    let spec_json = r#"{
        "title": "Favorite fruit?",
        "description": "Huh?",
        "poll_type": "single_choice",
        "nonce": 1498698199,
        "options": ["Apple", "Banana"]
    }"#;
    rig.send_message(NewPollMessage::SpecSet(spec_json.to_owned()));

    let title = select_single_element(&rig.root_element, "#title")
        .dyn_into::<HtmlInputElement>()
        .unwrap_throw()
        .value();
    assert_eq!(title, "Favorite fruit?");
    let description = select_single_element(&rig.root_element, "#description")
        .dyn_into::<HtmlTextAreaElement>()
        .unwrap_throw()
        .value();
    assert_eq!(description, "Huh?");
    let options: Vec<_> = select_elements(&rig.root_element, "input[id^=option-]")
        .map(|input| input.dyn_into::<HtmlInputElement>().unwrap_throw().value())
        .collect();
    assert_eq!(options, ["Apple", "Banana"]);

    let spec = extract_spec(&rig);
    assert_eq!(spec.description, "Huh?");
    let spec_col = input_col(&rig.root_element, "#poll-spec");
    assert_no_child(&spec_col, ".invalid-feedback");

    let invalid_spec_json = &spec_json[..30];
    rig.send_message(NewPollMessage::SpecSet(invalid_spec_json.to_owned()));

    let spec_json = select_single_element(&rig.root_element, "#poll-spec")
        .dyn_into::<HtmlTextAreaElement>()
        .unwrap_throw()
        .value();
    assert_eq!(spec_json, invalid_spec_json);
    let spec_col = input_col(&rig.root_element, "#poll-spec");
    let feedback = extract_feedback(&spec_col);
    assert!(
        feedback.contains("Error deserializing spec"),
        "{}",
        feedback
    );

    rig.send_message(NewPollMessage::SpecReset);
    let spec = extract_spec(&rig);
    assert_eq!(spec.description, "Huh?");
    let spec_col = input_col(&rig.root_element, "#poll-spec");
    assert_no_child(&spec_col, ".invalid-feedback");
}
