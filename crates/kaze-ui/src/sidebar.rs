//! The vertical tab sidebar. Subscribes to [`kaze_tabs::TabStore`] events
//! and renders rows accordingly — never the other way around (see
//! architecture doc §4: widgets are views of state, not the source of
//! it).
//!
//! Implementation note: v0.1 renders rows into a plain `GtkBox` and
//! diffs them on every `TabEvent`, rebuilding the row list. That's O(n)
//! per event rather than the fully virtualized `GtkListView` +
//! `gio::ListModel` approach described in the architecture doc — correct
//! and simple, but worth swapping to a real `gio::ListModel` adapter
//! before tab counts get large enough for the rebuild to be visible.
//! Tracked as a follow-up; not a blocker for v0.1.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ScrolledWindow};
use kaze_tabs::{TabId, TabStore};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Sidebar {
    pub root: GtkBox,
    row_list: GtkBox,
    on_activate: RefCell<Option<Box<dyn Fn(TabId)>>>,
    on_close: RefCell<Option<Box<dyn Fn(TabId)>>>,
    on_new_tab: RefCell<Option<Box<dyn Fn()>>>,
}

impl Sidebar {
    pub fn new() -> Rc<Self> {
        let root = GtkBox::new(Orientation::Vertical, 0);
        root.add_css_class("kaze-sidebar");

        let new_tab_button = Button::builder()
            .icon_name("list-add-symbolic")
            .css_classes(["flat"])
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .halign(Align::Fill)
            .build();
        root.append(&new_tab_button);

        let row_list = GtkBox::new(Orientation::Vertical, 2);
        let scroller = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&row_list)
            .build();
        root.append(&scroller);

        let sidebar = Rc::new(Self {
            root,
            row_list,
            on_activate: RefCell::new(None),
            on_close: RefCell::new(None),
            on_new_tab: RefCell::new(None),
        });

        let sb = sidebar.clone();
        new_tab_button.connect_clicked(move |_| {
            if let Some(cb) = sb.on_new_tab.borrow().as_ref() {
                cb();
            }
        });

        sidebar
    }

    pub fn on_activate(&self, f: impl Fn(TabId) + 'static) {
        *self.on_activate.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_close(&self, f: impl Fn(TabId) + 'static) {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_new_tab(&self, f: impl Fn() + 'static) {
        *self.on_new_tab.borrow_mut() = Some(Box::new(f));
    }

    /// Rebuild the row list from the current `TabStore` state. Called
    /// once at startup and again on every `TabEvent`.
    pub fn sync(self: &Rc<Self>, store: &TabStore) {
        while let Some(child) = self.row_list.first_child() {
            self.row_list.remove(&child);
        }

        let active_id = store.active_id();

        for tab in store.tabs() {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.add_css_class("kaze-tab-row");
            if Some(tab.id) == active_id {
                row.add_css_class("active");
            }

            let title = if tab.title.is_empty() { &tab.url } else { &tab.title };
            let label = Label::builder()
                .label(title)
                .halign(Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();
            row.append(&label);

            if tab.profile.is_incognito() {
                let badge = Label::new(Some("🕶"));
                row.append(&badge);
            }

            let close_button = Button::builder()
                .icon_name("window-close-symbolic")
                .css_classes(["flat", "circular"])
                .build();
            row.append(&close_button);

            let click = gtk4::GestureClick::new();
            let sidebar = self.clone();
            let id = tab.id;
            click.connect_pressed(move |_, _, _, _| {
                if let Some(cb) = sidebar.on_activate.borrow().as_ref() {
                    cb(id);
                }
            });
            row.add_controller(click);

            let sidebar = self.clone();
            close_button.connect_clicked(move |_| {
                if let Some(cb) = sidebar.on_close.borrow().as_ref() {
                    cb(id);
                }
            });

            self.row_list.append(&row);
        }
    }
}
