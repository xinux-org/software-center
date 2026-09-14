use gettextrs::gettext;
use log::warn;
use relm4::{
    ComponentParts, ComponentSender, RelmListBoxExt, RelmWidgetExt, SimpleComponent,
    adw::{self, prelude::*},
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    factory::FactoryVecDeque,
    gtk,
};
use std::{collections::HashSet, convert::identity};

use crate::ui::{
    installed::components::installed_item::InstalledItem,
    package::package_page::{PackagePageInit, PackagePageModel},
    window::*,
};

use super::components::search_item::{SearchItem, SearchItemModel};

#[tracker::track]
#[derive(Debug)]
pub struct SearchPageModel {
    navigation: adw::NavigationView,

    system_packages_type: SystemPkgs,

    #[tracker::no_eq]
    searchitems: FactoryVecDeque<SearchItemModel>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,
}

#[derive(Debug)]
pub enum SearchPageMsg {
    Search(Vec<SearchItem>),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    OpenRow(usize),
    OpenPackage(String),
}

#[relm4::component(pub)]
impl SimpleComponent for SearchPageModel {
    type Init = SystemPkgs;
    type Input = SearchPageMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
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
        system_packages_type: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            SearchPageMsg::UpdateInstalledPackages {
                system_packages: state.installed_user_packages.clone(),
                user_packages: state.installed_system_packages.clone(),
            }
        });

        let model = SearchPageModel {
            navigation: adw::NavigationView::new(),
            system_packages_type,
            searchitems: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .detach(),
            package_page: None,
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
            }
            SearchPageMsg::OpenRow(row) => {
                let searchitem_guard = self.searchitems.guard();
                if let Some(item) = searchitem_guard.get(row) {
                    let pkg = &item.item.pkg;
                    sender.input(SearchPageMsg::OpenPackage(pkg.to_string()));
                }
            }
            SearchPageMsg::OpenPackage(package) => {
                let package_page = PackagePageModel::builder()
                    .launch(PackagePageInit {
                        package,
                        syspkgs: self.system_packages_type.clone(),
                    })
                    .forward(sender.output_sender(), identity);
                self.navigation.push(package_page.widget());
                self.set_package_page(Some(package_page));
            }
            SearchPageMsg::UpdateInstalledPackages {
                system_packages,
                user_packages,
            } => {
                let system_packages = system_packages
                    .iter()
                    .map(|item| &item.pkg)
                    .collect::<HashSet<_>>();
                let user_packages = user_packages
                    .iter()
                    .map(|item| &item.pkg)
                    .collect::<HashSet<_>>();

                let mut guard = self.searchitems.guard();

                for item_model in guard.iter_mut() {
                    let item = item_model.get_mut_item();
                    item.installedsystem = system_packages.contains(&item.pkg);
                    item.installeduser = user_packages.contains(&item.pkg);
                }
            }
        }
    }
}
