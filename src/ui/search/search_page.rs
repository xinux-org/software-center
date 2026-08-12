use gettextrs::gettext;
use log::*;
use relm4::{
    adw::{self, prelude::*},
    factory::*,
    *,
};
use std::collections::HashSet;

use crate::ui::{
    search::components::search_item::{SearchItem, SearchItemModel},
    window::*,
};

#[tracker::track]
#[derive(Debug)]
pub struct SearchPageModel {
    #[tracker::no_eq]
    searchitems: FactoryVecDeque<SearchItemModel>,
    searchitemtracker: u8,
}

#[derive(Debug)]
pub enum SearchPageMsg {
    Search(Vec<SearchItem>),
    UpdateInstalled(HashSet<String>, HashSet<String>),
    OpenRow(usize),
    Noop,
}

#[relm4::component(pub)]
impl SimpleComponent for SearchPageModel {
    type Init = ();
    type Input = SearchPageMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(SearchPageModel::searchitemtracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            if !model.searchitems.is_empty() {
                adw::Clamp {
                    gtk::Stack {
                        set_transition_type: gtk::StackTransitionType::Crossfade,
                        set_margin_all: 20,
                        #[local_ref]
                        searchlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(SearchPageMsg::OpenRow(i as usize))
                                }
                            }
                        }
                    }
                }
            } else {
                adw::StatusPage {
                    set_icon_name: Some("edit-find-symbolic"),
                    set_title: &gettext("No apps found"),
                }
            }
        }
    }

    fn init(
        (): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SearchPageModel {
            searchitems: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .forward(sender.input_sender(), |_| SearchPageMsg::Noop),
            searchitemtracker: 0,
            tracker: 0,
        };

        let searchlist = model.searchitems.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            SearchPageMsg::Search(items) => {
                let mut searchitem_guard = self.searchitems.guard();
                searchitem_guard.clear();
                for item in items {
                    searchitem_guard.push_back(item);
                }
                searchitem_guard.drop();
                self.update_searchitemtracker(|_| ());
            }
            SearchPageMsg::OpenRow(row) => {
                let searchitem_guard = self.searchitems.guard();
                if let Some(item) = searchitem_guard.get(row) {
                    let pkg = &item.item.pkg;
                    let _ = sender.output(AppMsg::OpenPkg(pkg.to_string()));
                }
            }
            SearchPageMsg::UpdateInstalled(installeduserpkgs, installedsystempkgs) => {
                let mut searchitem_guard = self.searchitems.guard();
                for i in 0..searchitem_guard.len() {
                    if let Some(item) = searchitem_guard.get_mut(i) {
                        let pkgitem = item.get_mut_item();
                        pkgitem.installeduser =
                            installeduserpkgs.contains(&pkgitem.pname.to_string());
                        pkgitem.installedsystem =
                            installedsystempkgs.contains(&pkgitem.pkg.to_string());
                    }
                }
            }
            SearchPageMsg::Noop => {}
        }
    }
}
