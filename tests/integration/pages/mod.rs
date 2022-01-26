//! Tests for application pages.

use base64ct::{Base64UrlUnpadded, Encoding};
use js_sys::{Error, Promise, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::Element;
use yew::{html, Callback, Component, Context, ContextProvider, Html};

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use elastic_elgamal_site::{
    js::{ExportedData, ManageModals, PasswordBasedCrypto},
    pages::AppProperties,
    poll::SecretManager,
    testing::{ComponentRef, WithComponentRef},
};

mod new_poll;

#[derive(Debug)]
struct MockCrypto;

impl PasswordBasedCrypto for MockCrypto {
    fn seal(&self, password: &str, secret_bytes: &[u8]) -> Promise {
        assert_eq!(password, "correct horse battery staple");
        let encoded = Base64UrlUnpadded::encode_string(secret_bytes);
        Promise::resolve(&encoded.into())
    }

    fn cached(&self) -> Promise {
        Promise::resolve(&JsValue::null())
    }

    fn open(&self, password: &str, encrypted: &str) -> Promise {
        if password == "correct horse battery staple" {
            let decoded = Base64UrlUnpadded::decode_vec(encrypted).unwrap_throw();
            Promise::resolve(&Uint8Array::from(decoded.as_slice()).into())
        } else {
            Promise::reject(&Error::new("invalid password"))
        }
    }
}

#[derive(Debug)]
struct MockModals;

impl ManageModals for MockModals {
    fn hide_modal(&self, _: &str) {
        // do nothing
    }
}

#[derive(Debug)]
struct Calls<T> {
    calls: RefCell<Vec<T>>,
}

impl<T> Default for Calls<T> {
    fn default() -> Self {
        Self {
            calls: RefCell::default(),
        }
    }
}

impl<T> Calls<T> {
    fn push_call(&self, call_args: T) {
        self.calls.borrow_mut().push(call_args);
    }

    fn assert_called_once(&self) -> T {
        let mut calls = self.calls.borrow_mut();
        assert_eq!(calls.len(), 1, "not called once");
        calls.pop().unwrap_throw()
    }
}

#[derive(Debug)]
struct Wrapper<C: Component> {
    app_props: AppProperties,
    export_calls: Rc<Calls<ExportedData>>,
    _component: PhantomData<C>,
}

impl<C> Component for Wrapper<C>
where
    C: Component,
    <C as Component>::Properties: WithComponentRef<C>,
{
    type Message = ();
    type Properties = C::Properties;

    fn create(_: &Context<Self>) -> Self {
        let mock_crypto = Rc::new(MockCrypto);
        let mock_modals = Rc::new(MockModals);
        let export_calls = Rc::new(Calls::default());
        let export_calls_ = Rc::clone(&export_calls);

        Self {
            app_props: AppProperties {
                secrets: Rc::new(SecretManager::new(mock_crypto)),
                modals: mock_modals,
                onexport: Callback::from(move |data| export_calls_.push_call(data)),
            },
            export_calls,
            _component: PhantomData,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props().clone();
        let app_props = self.app_props.clone();

        html! {
            <ContextProvider<AppProperties> context={app_props}>
                <C ..props />
            </ContextProvider<AppProperties>>
        }
    }
}

struct TestRig<C: Component> {
    root_element: Element,
    export_calls: Rc<Calls<ExportedData>>,
    component: ComponentRef<C>,
}

impl<C: Component> Drop for TestRig<C> {
    // Remove the `root_element` from the document.
    fn drop(&mut self) {
        if let Some(parent) = self.root_element.parent_element() {
            if let Err(err) = parent.remove_child(self.root_element.as_ref()) {
                eprintln!("Error disposing root element for test rig: {:?}", err);
            }
        }
    }
}

impl<C> TestRig<C>
where
    C: Component,
    C::Properties: WithComponentRef<C>,
{
    fn new(mut props: C::Properties) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let div = document.create_element("div").unwrap();
        document.body().unwrap().append_with_node_1(&div).unwrap();

        let component = ComponentRef::<C>::default();
        props.set_component_ref(component.clone());
        let app_handle = yew::start_app_with_props_in_element::<Wrapper<C>>(div.clone(), props);
        let export_calls = &app_handle
            .get_component()
            .expect_throw("cannot get wrapper")
            .export_calls;

        Self {
            root_element: div,
            export_calls: Rc::clone(export_calls),
            component,
        }
    }

    fn send_message(&self, message: C::Message) {
        self.component.send_message(message);
    }

    fn export_calls(&self) -> &Calls<ExportedData> {
        &self.export_calls
    }
}

fn assert_no_child(root: &Element, selector: &str) {
    let selected = root.query_selector(selector).unwrap_or_else(|err| {
        panic!("Cannot query `{}` from {:?}: {:?}", selector, root, err);
    });
    if let Some(selected) = selected {
        panic!("Unexpected element `{}`: {:?}", selector, selected);
    }
}

fn select_elements(root: &Element, selector: &str) -> impl Iterator<Item = Element> {
    let nodes = root
        .query_selector_all(selector)
        .unwrap_or_else(|e| panic!("Querying elements `{}` failed: {:?}", selector, e));

    (0..nodes.length()).filter_map(move |i| nodes.get(i).unwrap().dyn_into::<Element>().ok())
}

fn select_single_element(root: &Element, selector: &str) -> Element {
    let mut iter = select_elements(root, selector);
    let first = iter.next();
    let second = iter.next();

    match (first, second) {
        (None, _) => panic!("`{}` did not match any elements in {:?}", selector, root),
        (Some(_), Some(_)) => panic!("`{}` matched multiple elements in {:?}", selector, root),
        (Some(single), None) => single,
    }
}

/// Extracts `.invalid-feedback` from an element.
fn extract_feedback(element: &Element) -> String {
    let feedback = element
        .query_selector(".invalid-feedback")
        .unwrap_throw()
        .expect_throw("no invalid feedback");
    feedback.text_content().unwrap()
}
