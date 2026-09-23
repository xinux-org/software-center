use gettextrs::gettext;
use log::debug;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    adw::{self, prelude::*},
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    factory::FactoryVecDeque,
    gtk,
};
use std::{collections::HashSet, convert::identity};

use crate::ui::{
    explore::components::carousel::CarouselInput,
    installed::components::installed_item::InstalledItem,
    package::{
        components::package_tile::{PkgTile, PkgTileMsg},
        package_page::{PackagePageInit, PackagePageModel},
    },
    window::{AppMsg, INSTALLED_PACKAGES_STATE, NIX_DATA_CONFIG_STATE, SystemPkgs},
};

use super::components::carousel::CarouselModel;

#[tracker::track]
#[derive(Debug)]
pub struct ExplorePageModel {
    navigation: adw::NavigationView,

    system_pkg_type: SystemPkgs,
    config: NixDataConfig,

    #[tracker::no_eq]
    recommended_apps: FactoryVecDeque<PkgTile>,

    #[tracker::no_eq]
    featured_carousel: Controller<CarouselModel>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,
}

#[derive(Debug)]
pub enum ExplorePageMsg {
    OpenPackage(String),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    UpdateRecommendedPackages(Vec<PkgTile>),
}

#[relm4::component(pub)]
impl SimpleComponent for ExplorePageModel {
    type Init = SystemPkgs;
    type Input = ExplorePageMsg;
    type Output = AppMsg;
    type Widgets = ExplorePageWidgets;

    view! {
        #[name = "navigation"]
        adw::NavigationView {
            add = &adw::NavigationPage {
                set_title: &gettext("Explore"),
                adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {},
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        adw::Clamp {
                            set_maximum_size: 1450,
                            set_tightening_threshold: 950,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_valign: gtk::Align::Start,
                                set_margin_top: 15,
                                set_spacing: 4,

                                #[local_ref]
                                featured_carousel -> gtk::Box {},

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
                                recommended_box -> gtk::FlowBox {
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
                            }
                        }
                    },
                },
            },
        },
    }

    fn init(
        system_pkg_type: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            ExplorePageMsg::UpdateInstalledPackages {
                system_packages: state.installed_system_packages.clone(),
                user_packages: state.installed_user_packages.clone(),
            }
        });

        let config = NIX_DATA_CONFIG_STATE.read().clone();

        let featured_carousel = CarouselModel::builder().launch(()).detach();

        let mut model = ExplorePageModel {
            navigation: adw::NavigationView::new(),

            system_pkg_type,
            config,

            recommended_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),

            featured_carousel,

            package_page: None,

            tracker: 0,
        };

        let recommended_box = model.recommended_apps.widget();

        let featured_carousel = model.featured_carousel.widget();

        let widgets = view_output!();

        model.navigation = widgets.navigation.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            ExplorePageMsg::OpenPackage(package) => {
                let package_page = PackagePageModel::builder()
                    .launch(PackagePageInit {
                        package,
                        syspkgs: self.system_pkg_type.clone(),
                    })
                    .forward(sender.output_sender(), identity);
                self.navigation.push(package_page.widget());
                self.set_package_page(Some(package_page));
            }

            ExplorePageMsg::UpdateInstalledPackages {
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

                debug!("Getting recommended apps guard");
                let mut recommended_apps_guard = self.recommended_apps.guard();
                debug!("Got recommended apps guard");
                for item in recommended_apps_guard.iter_mut() {
                    debug!("Got item {}", item.pkg);
                    item.installedsystem = system_packages.contains(&item.pkg);
                    item.installeduser = user_packages.contains(&item.pkg);
                }
            }

            ExplorePageMsg::UpdateRecommendedPackages(pkgtiles) => {
                // let packages = pkgtiles.iter().map(|tile| tile.pkg.clone()).collect();

                let mut guard = self.recommended_apps.guard();
                guard.clear();
                for tile in &pkgtiles {
                    guard.push_back(tile.clone());
                }

                self.featured_carousel
                    .emit(CarouselInput::SetPackages(pkgtiles));
            }
        }
    }
}
