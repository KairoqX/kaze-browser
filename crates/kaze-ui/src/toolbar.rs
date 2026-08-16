//! Top toolbar: back/forward/reload buttons + address bar. Emits intents
//! upward via callbacks rather than touching `TabStore` or the engine
//! directly — see the unidirectional-flow pattern in architecture doc §4.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Toolbar {
    pub root: GtkBox,
    pub address_entry: Entry,
    back_button: Button,
    forward_button: Button,
    reload_button: Button,
    on_navigate: RefCell<Option<Box<dyn Fn(String)>>>,
    on_back: RefCell<Option<Box<dyn Fn()>>>,
    on_forward: RefCell<Option<Box<dyn Fn()>>>,
    on_reload: RefCell<Option<Box<dyn Fn()>>>,
}

impl Toolbar {
    pub fn new() -> Rc<Self> {
        let root = GtkBox::new(Orientation::Horizontal, 6);
        root.set_margin_top(6);
        root.set_margin_bottom(6);
        root.set_margin_start(6);
        root.set_margin_end(6);

        let back_button = Button::from_icon_name("go-previous-symbolic");
        let forward_button = Button::from_icon_name("go-next-symbolic");
        let reload_button = Button::from_icon_name("view-refresh-symbolic");

        let address_entry = Entry::builder()
            .hexpand(true)
            .placeholder_text("Search or enter address")
            .css_classes(["kaze-address-bar"])
            .build();

        root.append(&back_button);
        root.append(&forward_button);
        root.append(&reload_button);
        root.append(&address_entry);

        let toolbar = Rc::new(Self {
            root,
            address_entry: address_entry.clone(),
            back_button,
            forward_button,
            reload_button,
            on_navigate: RefCell::new(None),
            on_back: RefCell::new(None),
            on_forward: RefCell::new(None),
            on_reload: RefCell::new(None),
        });

        let t = toolbar.clone();
        address_entry.connect_activate(move |entry| {
            if let Some(cb) = t.on_navigate.borrow().as_ref() {
                cb(entry.text().to_string());
            }
        });

        let t = toolbar.clone();
        toolbar.back_button.connect_clicked(move |_| {
            if let Some(cb) = t.on_back.borrow().as_ref() {
                cb();
            }
        });

        let t = toolbar.clone();
        toolbar.forward_button.connect_clicked(move |_| {
            if let Some(cb) = t.on_forward.borrow().as_ref() {
                cb();
            }
        });

        let t = toolbar.clone();
        toolbar.reload_button.connect_clicked(move |_| {
            if let Some(cb) = t.on_reload.borrow().as_ref() {
                cb();
            }
        });

        toolbar
    }

    pub fn on_navigate(&self, f: impl Fn(String) + 'static) {
        *self.on_navigate.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_back(&self, f: impl Fn() + 'static) {
        *self.on_back.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_forward(&self, f: impl Fn() + 'static) {
        *self.on_forward.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_reload(&self, f: impl Fn() + 'static) {
        *self.on_reload.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_address(&self, url: &str) {
        // Don't stomp the user's in-progress typing if an async engine
        // event (title/url/favicon changed) fires while they're editing —
        // this was the cause of "typing in the address bar does nothing":
        // WebKit events kept resetting the entry back to the old URL mid-keystroke.
        if self.address_entry.has_focus() {
            return;
        }
        self.address_entry.set_text(url);
    }
    pub fn set_nav_state(&self, can_back: bool, can_forward: bool) {
        self.back_button.set_sensitive(can_back);
        self.forward_button.set_sensitive(can_forward);
    }
}
