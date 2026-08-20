use adw::prelude::*;
use gettextrs::gettext;
use log::debug;
use relm4::prelude::*;
use std::collections::HashSet;

use crate::ui::{
    category::components::categories::{PkgCategory, PkgCategoryMsg, PkgGroup},
    installed::components::installed_item::InstalledItem,
    package::components::package_tile::{PkgTile, PkgTileMsg},
    window::{AppMsg, INSTALLED_PACKAGES_STATE, SystemPkgs},
};

#[tracker::track]
#[derive(Debug)]
pub struct ExplorePageModel {
    system_pkg_type: SystemPkgs,

    #[tracker::no_eq]
    recommended_apps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    development_apps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    games: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    graphic_apps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    web_apps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    video_apps: FactoryVecDeque<PkgTile>,
    #[tracker::no_eq]
    categories: FactoryVecDeque<PkgGroup>,
}

#[derive(Debug)]
pub enum ExplorePageMsg {
    OpenPackage(String),
    OpenCategory(PkgCategory),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    UpdateRecommendedPackages(Vec<PkgTile>, Option<PkgCategory>),
}

#[relm4::component(pub)]
impl SimpleComponent for ExplorePageModel {
    type Init = SystemPkgs;
    type Input = ExplorePageMsg;
    type Output = AppMsg;
    type Widgets = ExplorePageWidgets;

    view! {
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
                    gtk::Label {
                        add_css_class: "title-1",
                        set_label: &gettext("Categories"),
                        set_halign: gtk::Align::Start,
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    category_box -> gtk::FlowBox {
                        set_halign: gtk::Align::Fill,
                        set_hexpand: true,
                        set_valign: gtk::Align::Center,
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
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "title-1",
                        set_label: &gettext("Development"),
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    development_box -> gtk::FlowBox {
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
                        set_label: &gettext("Game"),
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    games_box -> gtk::FlowBox {
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
                        set_label: &gettext("Graphic"),
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    graphic_box -> gtk::FlowBox {
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
                        set_label: &gettext("Web"),
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    web_box -> gtk::FlowBox {
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
                        set_label: &gettext("Video"),
                        set_margin_bottom: 6,
                        set_margin_end: 3,
                        set_margin_start: 3,
                        set_margin_top: 0
                    },
                    #[local_ref]
                    video_box -> gtk::FlowBox {
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

        let categories = [
            PkgCategory::Audio,
            PkgCategory::Development,
            PkgCategory::Games,
            PkgCategory::Graphics,
            PkgCategory::Web,
            PkgCategory::Video,
            PkgCategory::Education,
            PkgCategory::Science,
            PkgCategory::Office,
            PkgCategory::Network,
            PkgCategory::System,
            PkgCategory::Utility,
        ];

        let mut category_factory = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.input_sender(), |category| match category {
                PkgCategoryMsg::Open(category) => ExplorePageMsg::OpenCategory(category),
            });
        {
            let mut guard = category_factory.guard();
            for category in categories {
                guard.push_back(category);
            }
        }

        let model = ExplorePageModel {
            system_pkg_type,

            recommended_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),
            development_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),
            games: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),
            graphic_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),
            web_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),
            video_apps: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(sender.input_sender(), |pkg_tile_msg| match pkg_tile_msg {
                    PkgTileMsg::Open(x) => ExplorePageMsg::OpenPackage(x),
                }),

            categories: category_factory,

            tracker: 0,
        };

        let recommended_box = model.recommended_apps.widget();
        let development_box = model.development_apps.widget();
        let games_box = model.games.widget();
        let graphic_box = model.graphic_apps.widget();
        let web_box = model.web_apps.widget();
        let video_box = model.video_apps.widget();
        let category_box = model.categories.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            ExplorePageMsg::OpenPackage(package) => {
                sender.output(AppMsg::OpenPkg(package));
            }
            ExplorePageMsg::OpenCategory(category) => {
                sender.output(AppMsg::OpenCategoryPage(category));
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

            ExplorePageMsg::UpdateRecommendedPackages(pkgtiles, pkg_category) => {
                let mut guard = match pkg_category {
                    Some(PkgCategory::Audio) => todo!(),
                    Some(PkgCategory::Development) => self.development_apps.guard(),
                    Some(PkgCategory::Games) => self.games.guard(),
                    Some(PkgCategory::Graphics) => self.graphic_apps.guard(),
                    Some(PkgCategory::Web) => self.web_apps.guard(),
                    Some(PkgCategory::Video) => self.video_apps.guard(),
                    Some(PkgCategory::Education) => todo!(),
                    Some(PkgCategory::Science) => todo!(),
                    Some(PkgCategory::Office) => todo!(),
                    Some(PkgCategory::Network) => todo!(),
                    Some(PkgCategory::System) => todo!(),
                    Some(PkgCategory::Utility) => todo!(),
                    _ => self.recommended_apps.guard(),
                };

                guard.clear();

                for tile in pkgtiles {
                    guard.push_back(tile.clone());
                }
            }
        }
    }
}
