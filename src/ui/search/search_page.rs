use gettextrs::gettext;
use log::debug;
use relm4::{
    Component, ComponentParts, ComponentSender, RelmListBoxExt, RelmWidgetExt,
    adw::{self, prelude::*},
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    factory::FactoryVecDeque,
    gtk,
};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::{collections::HashSet, convert::identity};

use crate::ui::{
    installed::components::installed_item::InstalledItem,
    package::package_page::{PackagePageInit, PackagePageModel},
    window::{AppMsg, INSTALLED_PACKAGES_STATE, SystemPkgs},
    windowloading::{APPSTREAM_DATA_STATE, PACKAGES_DB_STATE},
};

use super::components::search_item::{SearchItem, SearchItemModel};

#[tracker::track]
#[derive(Debug)]
pub struct SearchPageModel {
    navigation: adw::NavigationView,

    system_packages_type: SystemPkgs,

    installed_system_packages: HashSet<String>,
    installed_user_packages: HashSet<String>,

    search_text: String,
    searching: bool,

    #[tracker::no_eq]
    search_results: FactoryVecDeque<SearchItemModel>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,
}

#[derive(Debug)]
pub enum SearchPageMsg {
    Search(String),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    OpenRow(usize),
    OpenPackage(String),
}

#[derive(Debug)]
pub enum SearchPageAsyncMessage {
    SetResults(String, Vec<SearchItem>),
}

#[relm4::component(pub)]
impl Component for SearchPageModel {
    type Init = SystemPkgs;
    type Input = SearchPageMsg;
    type Output = AppMsg;
    type CommandOutput = SearchPageAsyncMessage;

    view! {
        #[name = "navigation"]
        adw::NavigationView {
            add = &adw::NavigationPage {
                set_title: &gettext("Search"),
                adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {},
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        adw::Clamp {
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_top: 15,
                                set_spacing: 4,

                                adw::Clamp {
                                    gtk::SearchEntry {
                                        set_placeholder_text: Some(&gettext("Search apps")),
                                        connect_search_changed[sender] => move |search_entry| {
                                            sender.input(SearchPageMsg::Search(search_entry.text().to_string()));
                                        },
                                    },
                                },

                                if !model.search_results.is_empty() {
                                    gtk::Stack {
                                        set_transition_type: gtk::StackTransitionType::Crossfade,
                                        set_margin_all: 20,
                                        #[local_ref]
                                        search_results_factory -> gtk::ListBox {
                                            set_valign: gtk::Align::Start,
                                            add_css_class: "boxed-list",
                                            set_selection_mode: gtk::SelectionMode::None,
                                            connect_row_activated[sender] => move |list_box, row| {
                                                if let Some(i) = list_box.index_of_child(row) {
                                                    sender.input(SearchPageMsg::OpenRow(i as usize));
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    gtk::Box {
                                        set_halign: gtk::Align::Fill,
                                        set_valign: gtk::Align::Fill,
                                        set_vexpand: true,
                                        if model.searching {
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Center,
                                                set_hexpand: true,
                                                set_vexpand: true,
                                                set_spacing: 6,
                                                gtk::Spinner {
                                                    set_spinning: true,
                                                    set_width_request: 64,
                                                    set_height_request: 64,
                                                    set_margin_bottom: 18,
                                                },
                                                gtk::Label {
                                                    set_label: &gettext("Searching..."),
                                                    set_wrap: true,
                                                    set_justify: gtk::Justification::Center,
                                                    set_margin_bottom: 24,
                                                    add_css_class: "title-3",
                                                },
                                            }
                                        } else {
                                            adw::StatusPage {
                                                set_hexpand: true,
                                                set_vexpand: true,
                                                set_icon_name: Some("edit-find-symbolic"),
                                                #[watch]
                                                set_title: &if model.search_text.is_empty() { gettext("Type to search") } else { gettext("No apps found") },
                                            }
                                        },
                                    }
                                },
                            }
                        }
                    },
                },
            },
        },
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

        let installed_packages = &INSTALLED_PACKAGES_STATE.read();
        let installed_system_packages: HashSet<String> = installed_packages
            .installed_system_packages
            .iter()
            .map(|item| item.pkg.clone())
            .collect::<HashSet<_>>();
        let installed_user_packages: HashSet<String> = installed_packages
            .installed_user_packages
            .iter()
            .map(|item| item.pkg.clone())
            .collect::<HashSet<_>>();

        let mut model = SearchPageModel {
            navigation: adw::NavigationView::new(),
            system_packages_type,
            installed_system_packages,
            installed_user_packages,
            search_text: String::new(),
            searching: false,
            search_results: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .detach(),
            package_page: None,
            tracker: 0,
        };

        let search_results_factory = model.search_results.widget();

        let widgets = view_output!();

        model.set_navigation(widgets.navigation.clone());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        match msg {
            SearchPageMsg::Search(search_text) => {
                debug!("SearchPageMsg::Search({search_text})");

                let installed_system_packages = self.installed_system_packages.clone();
                let installed_user_packages = self.installed_user_packages.clone();

                if search_text.len() < 2 {
                    debug!("search text '{search_text}' is too short - not searching");
                    self.set_searching(false);
                    self.set_search_text(String::new());
                    self.search_results.guard().clear();
                } else {
                    debug!("searching for '{search_text}'...");
                    self.set_searching(true);
                    self.set_search_text(search_text.clone());

                    sender.command(move |output_sender, shutdown| {
                        shutdown
                            .register(async move {
                                let results = search_packages(
                                    search_text.clone(),
                                    installed_system_packages,
                                    installed_user_packages,
                                )
                                .await;
                                debug!("{} results found for '{}'", results.len(), search_text);
                                output_sender
                                    .send(SearchPageAsyncMessage::SetResults(search_text, results));
                            })
                            .drop_on_shutdown()
                    });
                }
            }
            SearchPageMsg::OpenRow(row) => {
                let guard = self.search_results.guard();
                if let Some(item) = guard.get(row) {
                    let package = &item.item.pkg;
                    sender.input(SearchPageMsg::OpenPackage(package.clone()));
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
                let system_packages: HashSet<String> = system_packages
                    .iter()
                    .map(|item| item.pkg.clone())
                    .collect::<HashSet<_>>();
                let user_packages: HashSet<String> = user_packages
                    .iter()
                    .map(|item| item.pkg.clone())
                    .collect::<HashSet<_>>();

                self.set_installed_system_packages(system_packages.clone());
                self.set_installed_user_packages(user_packages.clone());

                let mut guard = self.search_results.guard();

                for item_model in guard.iter_mut() {
                    let item = item_model.get_mut_item();
                    item.installedsystem = system_packages.contains(&item.pkg);
                    item.installeduser = user_packages.contains(&item.pkg);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            SearchPageAsyncMessage::SetResults(search_text, items) => {
                if search_text == self.search_text {
                    debug!(
                        "{} search items received for search text '{}'",
                        items.len(),
                        search_text
                    );
                    let mut guard = self.search_results.guard();
                    guard.clear();
                    for item in items {
                        guard.push_back(item);
                    }
                    guard.drop();
                    self.set_searching(false);
                }
                {
                    debug!(
                        "'{}' results rececived, but currently searching for {}",
                        search_text, self.search_text
                    );
                }
            }
        }
    }
}

async fn search_packages(
    search_text: String,
    installed_system_packages: HashSet<String>,
    installed_user_packages: HashSet<String>,
) -> Vec<SearchItem> {
    let search_split: Vec<String> = search_text
        .split(' ')
        .filter(|x| x.len() > 1)
        .map(std::string::ToString::to_string)
        .collect();

    let packgages_db = PACKAGES_DB_STATE.read().packages_db.clone();

    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{packgages_db}")).await {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT pkgs.attribute, pkgs.pname, description, version FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE (",
        );
        for (i, q) in search_split.iter().enumerate() {
            query
                .push("pkgs.attribute LIKE ")
                .push_bind(format!("%{q}%"))
                .push(r" OR description LIKE ")
                .push_bind(format!("%{q}%"));
            if i == search_split.len() - 1 {
                query.push(")");
            } else {
                query.push(r") AND (");
            }
        }
        query.push("ORDER BY LENGTH(pkgs.attribute) ASC");
        let results: Vec<(String, String, String, String)> = query
            .build_query_as()
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        let app_data = APPSTREAM_DATA_STATE.read().clone();

        let mut results = results
            .into_iter()
            .take(200)
            .map(|(package, package_name, description, _version)| {
                let mut name = package_name.clone();
                let pkg = package.clone();
                let pname = package.clone();
                let summary = if description.is_empty() {
                    None
                } else {
                    Some(description)
                };
                let mut icon = None;
                let installeduser = installed_user_packages.contains(&package);
                let installedsystem = installed_system_packages.contains(&package);

                if let Some(data) = app_data.get(&package) {
                    if let Some(names) = &data.name
                        && let Some(name_) = names.get("C")
                    {
                        name.clone_from(name_);
                    }

                    icon = data
                        .icon
                        .as_ref()
                        .and_then(|icon_list| icon_list.cached.as_ref())
                        .and_then(|icons| icons.first())
                        .map(|icon| icon.name.clone());
                }

                SearchItem {
                    name,
                    pkg,
                    pname,
                    summary,
                    icon,
                    installeduser,
                    installedsystem,
                }
            })
            .collect::<Vec<_>>();

        results.sort_by(|a, b| {
            let mut a_left = a.name.to_lowercase() + &a.pkg.to_lowercase();
            let mut b_left = b.name.to_lowercase() + &b.pkg.to_lowercase();
            for q in &search_split {
                let q = &q.to_lowercase();
                if a_left.contains(q) {
                    a_left = a_left.replace(q, "");
                } else {
                    a_left.push_str(q);
                }
                if b_left.contains(q) {
                    b_left = b_left.replace(q, "");
                } else {
                    b_left.push_str(q);
                }
            }
            let mut a_points = a_left.len() + 5;
            let mut b_points = b_left.len() + 5;

            if app_data.contains_key(&a.pkg) {
                a_points -= 5;
            }
            if app_data.contains_key(&b.pkg) {
                b_points -= 5;
            }
            a_points.cmp(&b_points)
        });

        results
    } else {
        vec![]
    }
}
