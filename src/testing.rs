//! Test harness for Yew components.

use yew::{html::Scope, Component, Properties};

use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct ComponentRef<C: Component> {
    link: Rc<RefCell<Option<Scope<C>>>>,
}

impl<C: Component> Default for ComponentRef<C> {
    fn default() -> Self {
        Self {
            link: Rc::new(RefCell::new(None)),
        }
    }
}

impl<C: Component> Clone for ComponentRef<C> {
    fn clone(&self) -> Self {
        Self {
            link: Rc::clone(&self.link),
        }
    }
}

impl<C: Component> PartialEq for ComponentRef<C> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.link, &other.link)
    }
}

impl<C: Component> ComponentRef<C> {
    pub fn link_with(&self, link: Scope<C>) {
        *self.link.borrow_mut() = Some(link);
    }

    pub fn send_message(&self, message: C::Message) {
        if let Some(link) = self.link.borrow().as_ref() {
            link.send_message(message);
        }
    }
}

pub trait WithComponentRef<C: Component>: Properties + Clone {
    fn set_component_ref(&mut self, component_ref: ComponentRef<C>);
}
