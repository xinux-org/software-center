use std::{collections::HashSet, convert::identity};

use gettextrs::gettext;
use log::warn;
use relm4::{
    Component, ComponentParts, ComponentSender,
    adw::{self, prelude::*},
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    factory::FactoryVecDeque,
    gtk,
};

use crate::ui::{
    category::components::{
        categories::PkgCategory,
        category_tile::{CategoryTile, CategoryTileMsg},
    },
    installed::components::installed_item::InstalledItem,
    package::package_page::{PackagePageInit, PackagePageModel},
    window::{AppMsg, INSTALLED_PACKAGES_STATE, SystemPkgs},
};

#[derive(Debug)]
pub struct CategoryPageInit {
    pub category: PkgCategory,
    pub system_packages_type: SystemPkgs,
}

#[tracker::track]
#[derive(Debug)]
pub struct CategoryPageModel {
    navigation: adw::NavigationView,

    system_packages_type: SystemPkgs,

    category: PkgCategory,
    #[tracker::no_eq]
    recommended_apps: FactoryVecDeque<CategoryTile>,
    #[tracker::no_eq]
    all_apps: FactoryVecDeque<CategoryTile>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,

    busy: bool,
}

#[derive(Debug)]
pub enum CategoryPageMsg {
    OpenPackage(String),
    UpdatePackages(Vec<CategoryTile>, Vec<CategoryTile>),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
}

#[derive(Debug)]
pub enum CategoryPageAsyncMsg {
    ExtendApps(Vec<CategoryTile>),
}

#[relm4::component(pub)]
impl Component for CategoryPageModel {
    type Init = CategoryPageInit;
    type Input = CategoryPageMsg;
    type Output = AppMsg;
    type CommandOutput = CategoryPageAsyncMsg;

    view! {
        #[name = "navigation"]
        adw::NavigationView {
            add = &adw::NavigationPage {
                set_title: &match model.category {
                    PkgCategory::Audio => gettext("Audio"),
                    PkgCategory::Development => gettext("Development"),
                    PkgCategory::Games => gettext("Games"),
                    PkgCategory::Graphics => gettext("Graphics"),
                    PkgCategory::Web => gettext("Web"),
                    PkgCategory::Video => gettext("Video"),
                    PkgCategory::Education => gettext("Education"),
                    PkgCategory::Science => gettext("Science"),
                    PkgCategory::Office => gettext("Office"),
                    PkgCategory::Network => gettext("Network"),
                    PkgCategory::System => gettext("System"),
                    PkgCategory::Utility => gettext("Utility"),
                },

                adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {},
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_vscrollbar_policy: gtk::PolicyType::Automatic,
                        #[track(model.changed(CategoryPageModel::category()))]
                        set_vadjustment: gtk::Adjustment::NONE,
                        adw::Clamp {
                            set_maximum_size: 1450,
                            set_tightening_threshold: 950,
                            if model.busy {
                                #[name(spinner)]
                                gtk::Spinner {
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_halign: gtk::Align::Center,
                                    set_valign: gtk::Align::Center,
                                    set_spinning: true,
                                    set_size_request: (64, 64),
                                }
                            } else {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Start,
                                    set_margin_top: 15,
                                    set_spacing: 4,
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        add_css_class: "title-1",
                                        set_label: &gettext("Recommended"),
                                        set_margin_bottom: 6,
                                        set_margin_end: 3,
                                        set_margin_start: 3,
                                        set_margin_top: 0
                                    },
                                    #[local_ref]
                                    recommended_apps_factory -> gtk::FlowBox {
                                        set_halign: gtk::Align::Fill,
                                        set_valign: gtk::Align::Fill,
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_selection_mode: gtk::SelectionMode::None,
                                        set_homogeneous: true,
                                        set_max_children_per_line: 4,
                                        set_min_children_per_line: 1,
                                        set_column_spacing: 11,
                                        set_row_spacing: 11,
                                    },
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        add_css_class: "title-1",
                                        set_label: &gettext("Other"),
                                        set_margin_bottom: 6,
                                        set_margin_end: 3,
                                        set_margin_start: 3,
                                        set_margin_top: 0
                                    },
                                    #[local_ref]
                                    all_apps_factory -> gtk::FlowBox {
                                        set_halign: gtk::Align::Fill,
                                        set_valign: gtk::Align::Fill,
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_selection_mode: gtk::SelectionMode::None,
                                        set_homogeneous: true,
                                        set_max_children_per_line: 4,
                                        set_min_children_per_line: 1,
                                        set_column_spacing: 11,
                                        set_row_spacing: 11,
                                    }
                                }
                            }
                        }
                    }
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            CategoryPageMsg::UpdateInstalledPackages {
                system_packages: state.installed_system_packages.clone(),
                user_packages: state.installed_user_packages.clone(),
            }
        });

        let recommended_apps = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(
                sender.input_sender(),
                |category_tile_msg| match category_tile_msg {
                    CategoryTileMsg::Open(package) => CategoryPageMsg::OpenPackage(package),
                },
            );
        let all_apps = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(
                sender.input_sender(),
                |category_tile_msg| match category_tile_msg {
                    CategoryTileMsg::Open(package) => CategoryPageMsg::OpenPackage(package),
                },
            );

        let mut model = CategoryPageModel {
            navigation: adw::NavigationView::new(),
            system_packages_type: init.system_packages_type,
            category: init.category,
            recommended_apps,
            all_apps,
            package_page: None,
            busy: false,
            tracker: 0,
        };

        let recommended_apps_factory = model.recommended_apps.widget();
        let all_apps_factory = model.all_apps.widget();

        let widgets = view_output!();

        model.navigation = widgets.navigation.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.reset();
        match msg {
            CategoryPageMsg::OpenPackage(package) => {
                let package_page = PackagePageModel::builder()
                    .launch(PackagePageInit {
                        package,
                        syspkgs: self.system_packages_type.clone(),
                    })
                    .forward(sender.output_sender(), identity);
                self.navigation.push(package_page.widget());
                self.set_package_page(Some(package_page));
            }

            CategoryPageMsg::UpdatePackages(recommended_apps, all_apps) => {
                {
                    let mut guard = self.recommended_apps.guard();
                    guard.clear();
                    for tile in recommended_apps {
                        guard.push_back(tile);
                    }
                }

                {
                    let mut guard = self.all_apps.guard();
                    guard.clear();
                }

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            for tiles in all_apps.chunks(10) {
                                let _ = out.send(CategoryPageAsyncMsg::ExtendApps(tiles.to_vec()));
                                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                            }
                        })
                        .drop_on_shutdown()
                });
            }

            CategoryPageMsg::UpdateInstalledPackages {
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

                let mut guard = self.recommended_apps.guard();
                for tile in guard.iter_mut() {
                    tile.installed_system = system_packages.contains(&tile.package);
                    tile.installed_user = user_packages.contains(&tile.package);
                }

                let mut guard = self.all_apps.guard();
                for tile in guard.iter_mut() {
                    tile.installed_system = system_packages.contains(&tile.package);
                    tile.installed_user = user_packages.contains(&tile.package);
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
            CategoryPageAsyncMsg::ExtendApps(category_tiles) => {
                self.all_apps.extend(category_tiles);
            }
        }
    }
}
