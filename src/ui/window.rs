use gettextrs::gettext;
use log::*;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{
    self, AsyncComponentSender, Component, ComponentController, Controller, MessageBroker, Sender,
    SharedState, WorkerController,
    actions::{RelmAction, RelmActionGroup},
    adw::{self, prelude::*},
    component::AsyncController,
    gtk::{self},
    prelude::{AsyncComponent, AsyncComponentParts},
};
use sqlx::{Sqlite, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    fs,
    path::Path,
    time::Duration,
};

use crate::{
    config::{self, PROFILE},
    ui::{
        about::about_page::AboutPageModel,
        category::{
            category_page::{CategoryPageInit, CategoryPageModel, CategoryPageMsg},
            components::{categories::PkgCategory, category_tile::CategoryTile},
        },
        explore::explore_page::{ExplorePageModel, ExplorePageMsg},
        installed::{
            components::installed_item::InstalledItem,
            installed_page::{InstalledPageModel, InstalledPageMsg},
        },
        package::{
            components::package_tile::PkgTile,
            package_page::{InstallType, PackagePageModel, WorkPackage},
        },
        preferences::preferences_page::{PreferencesPageModel, PreferencesPageMsg},
        rebuild::rebuild_model::{RebuildModel, RebuildMsg},
        search::search_page::SearchPageModel,
        update::{
            components::update_item::UpdateItem,
            unavailable_dialog::{UnavailableDialogMsg, UnavailableItemModel},
            update_page::{
                UNAVAILABLE_BROKER, UpdatePageInit, UpdatePageModel, UpdatePageMsg, UpdateType,
            },
        },
        welcome::welcome_page::{WelcomeModel, WelcomeMsg},
        windowloading::{
            LoadErrorModel, LoadErrorMsg, PACKAGES_DB_STATE, WindowAsyncHandler,
            WindowAsyncHandlerMsg,
        },
    },
    utils::{
        cli,
        config::{editconfig, getconfig},
        online::{checkonline, checkonline_async},
        packages::AppData,
    },
};

pub static REBUILD_BROKER: MessageBroker<RebuildMsg> = MessageBroker::new();

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SystemPkgs {
    Flake,
    None,
}

#[derive(Default)]
pub struct InstalledPackagesState {
    pub installed_system_packages: Vec<InstalledItem>,
    pub installed_user_packages: Vec<InstalledItem>,
}

pub static INSTALLED_PACKAGES_STATE: SharedState<InstalledPackagesState> = SharedState::new();

pub static NIX_DATA_CONFIG_STATE: SharedState<NixDataConfig> = SharedState::new();

#[tracker::track]
pub struct AppModel {
    navigation: adw::NavigationSplitView,
    mainwindow: adw::ApplicationWindow,
    config: NixDataConfig,
    #[tracker::no_eq]
    windowloading: WorkerController<WindowAsyncHandler>,
    #[tracker::no_eq]
    loaderrordialog: Controller<LoadErrorModel>,
    busy: bool,
    // #[tracker::no_eq]
    // pkgs: HashMap<String, Package>,
    // syspkgs: HashMap<String, String>,
    // profilepkgs: Option<HashMap<String, String>>,
    // pkgitems: HashMap<String, PkgItem>,
    appdata: HashMap<String, AppData>,
    installeduserpkgs: HashMap<String, String>,
    installedsystempkgs: HashSet<String>,
    syspkgtype: SystemPkgs,
    recommended_apps: Vec<String>,
    category_apps_recommended: HashMap<PkgCategory, Vec<String>>,
    category_apps_all: HashMap<PkgCategory, Vec<String>>,

    #[tracker::no_eq]
    search_page: Controller<SearchPageModel>,

    #[tracker::no_eq]
    preferencespage: Controller<PreferencesPageModel>,
    #[tracker::no_eq]
    explore_page: Controller<ExplorePageModel>,

    #[tracker::no_eq]
    category_audio_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_development_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_games_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_graphics_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_web_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_video_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_education_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_science_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_office_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_network_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_system_page: Controller<CategoryPageModel>,
    #[tracker::no_eq]
    category_utility_page: Controller<CategoryPageModel>,

    #[tracker::no_eq]
    installed_page: Controller<InstalledPageModel>,
    #[tracker::no_eq]
    update_page: Controller<UpdatePageModel>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,
    viewstack: adw::ViewStack,
    installedpagebusy: Vec<(String, InstallType)>,
    #[tracker::no_eq]
    rebuild: Controller<RebuildModel>,
    #[tracker::no_eq]
    welcomepage: Controller<WelcomeModel>,
    online: bool,
    updates_count: usize,
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateSysconfig(Option<String>),
    UpdateFlake(Option<String>, Option<String>),
    TryLoad,
    UpdateDB,
    LoadConfig(NixDataConfig),
    Close,
    LoadError(String, String),
    Initialize(
        HashMap<String, AppData>,
        Vec<String>,
        HashMap<PkgCategory, Vec<String>>,
        HashMap<PkgCategory, Vec<String>>,
    ),
    OpenPkgByScheme(cli::scheme::Scheme),
    UpdateInstalledPkgs,
    UpdateInstalledPage,
    AddInstalledToWorkQueue(WorkPackage),
    RemoveInstalledBusy(WorkPackage),
    LoadRecommended,
    LoadCategory(PkgCategory),
    SetDarkMode(bool),
    GetUnavailableItems(HashMap<String, String>, HashMap<String, String>, UpdateType),
    CheckNetwork,
    ShowPreferences,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkgItem {
    pkg: String,
    pname: String,
    name: String,
    version: String,
    summary: Option<String>,
    icon: Option<String>,
}

#[derive(Debug)]
pub enum AppAsyncMsg {
    UpdateInstalledPkgs(HashSet<String>, HashMap<String, String>),
    LoadRecommended(Vec<PkgTile>),
    LoadCategory(PkgCategory, Vec<CategoryTile>, Vec<CategoryTile>),
    SetNetwork(bool),
}

#[relm4::component(pub, async)]
impl AsyncComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppAsyncMsg;
    type Widgets = AppWidgets;

    view! {
        #[root]
        #[name(main_window)]
        adw::ApplicationWindow {
            set_default_width: 1220,
            set_default_height: 900,
            set_width_request: 360,
            set_height_request: 294,

            add_css_class?: if PROFILE == "Devel" {
                    Some("devel")
                } else {
                    None
                },

            #[name = "navigation"]
            adw::NavigationSplitView {
                #[wrap(Some)]
                set_sidebar = &adw::NavigationPage {
                    set_title: &gettext("Settings"),
                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            pack_end = &gtk::MenuButton {
                                set_icon_name: "open-menu-symbolic",
                                set_menu_model: Some(&mainmenu),
                            }
                        },
                        #[wrap(Some)]
                        set_content = &adw::ViewSwitcherSidebar {
                            set_stack: Some(&view_stack),
                        },
                    },
                },

                #[wrap(Some)]
                set_content = &adw::NavigationPage {
                    #[wrap(Some)]
                    set_child = if !model.busy {
                        adw::ToolbarView {
                            set_content: Some(&view_stack),
                        }
                    } else {
                        adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {},
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
                                    set_label: &gettext("Refreshing..."),
                                    set_wrap: true,
                                    set_justify: gtk::Justification::Center,
                                    set_margin_bottom: 24,
                                    add_css_class: "title-3",
                                },
                            },
                        }
                    },
                },
            },
        },

        view_stack = &adw::ViewStack {
            add_titled_with_icon: (model.explore_page.widget(), Some("explore"), &gettext("Explore"), "compass2-symbolic"),

            add_titled_with_icon: (model.search_page.widget(), Some("search"), &gettext("Search"), "edit-find-symbolic"),

            add_titled_with_icon: (model.category_audio_page.widget(), Some("audio"), &gettext("Audio"), "headphones-symbolic"),
            add_titled_with_icon: (model.category_development_page.widget(), Some("development"), &gettext("Development"), "code-symbolic"),
            add_titled_with_icon: (model.category_games_page.widget(), Some("gaming"), &gettext("Gaming"), "gamepad-symbolic"),
            add_titled_with_icon: (model.category_graphics_page.widget(), Some("graphics"), &gettext("Graphics"), "paintbrush-symbolic"),
            add_titled_with_icon: (model.category_video_page.widget(), Some("video"), &gettext("Video"), "video-camera-symbolic"),
            add_titled_with_icon: (model.category_web_page.widget(), Some("web"), &gettext("Web"), "globe-alt2-symbolic"),
            add_titled_with_icon: (model.category_network_page.widget(), Some("network"), &gettext("Network"), "network-server-symbolic"),
            add_titled_with_icon: (model.category_education_page.widget(), Some("education"), &gettext("Education"), "school-symbolic"),
            add_titled_with_icon: (model.category_science_page.widget(), Some("science"), &gettext("Science"), "action-unavailable-symbolic"),
            add_titled_with_icon: (model.category_office_page.widget(), Some("office"), &gettext("Office"), "action-unavailable-symbolic"),
            add_titled_with_icon: (model.category_system_page.widget(), Some("system"), &gettext("System"), "settings-symbolic"),
            add_titled_with_icon: (model.category_utility_page.widget(), Some("utility"), &gettext("Utility"), "build-alt-symbolic"),

            add_titled_with_icon: (model.installed_page.widget(), Some("installed"), &gettext("Installed"), "library-symbolic"),
            add_titled_with_icon: (model.update_page.widget(), Some("updates"), &gettext("Updates"), "nsc-update-symbolic"),
        }
    }

    menu! {
        mainmenu: {
            &gettext("Preferences") => PreferencesAction,
            &gettext("About") => AboutAction,
        }
    }

    // #[tokio::main]
    async fn init(
        _application: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let (config, welcome) = if let Some(config) = getconfig() {
            debug!("Got config: {:?}", config);
            let mut out = false;
            if let Some(configpath) = &config.systemconfig
                && !Path::new(configpath).exists()
            {
                warn!("Invalid system config path: {}", configpath);
                out = true
            }
            if let Some(flakepath) = &config.flake
                && !Path::new(&flakepath).exists()
            {
                warn!("Invalid flake path: {}", flakepath);
                out = true
            }
            (config, out)
        } else {
            // Show welcome page
            debug!("No config found");
            (
                NixDataConfig {
                    systemconfig: None,
                    flake: None,
                    flakearg: None,
                    generations: None,
                },
                true,
            )
        };

        *NIX_DATA_CONFIG_STATE.write() = config.clone();

        let nixos = Path::new("/etc/nixos").exists();
        let syspkgtype = if config.systemconfig.is_none() || !nixos {
            SystemPkgs::None
        } else {
            match fs::read_to_string("/run/current-system/nixos-version") {
                Ok(s) => {
                    if !Path::new("/nix/var/nix/profiles/per-user/root/channels/nixos").exists()
                        || config.flake.is_some()
                    {
                        SystemPkgs::Flake
                    } else if let Some(last) = s.split('.').next_back()
                        && (last.len() == 7 || last == "dirty" || last == "git")
                    {
                        SystemPkgs::Flake
                    } else {
                        SystemPkgs::None
                    }
                }
                Err(_) => SystemPkgs::None,
            }
        };

        debug!("syspkgtype: {:?}", syspkgtype);

        let online = checkonline();

        let windowloading = WindowAsyncHandler::builder()
            .detach_worker(())
            .forward(sender.input_sender(), identity);
        let loaderrordialog = LoadErrorModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let preferencespage = PreferencesPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);

        let search_page = SearchPageModel::builder()
            .launch(syspkgtype.clone())
            .forward(sender.input_sender(), identity);

        let explore_page = ExplorePageModel::builder()
            .launch(syspkgtype.clone())
            .forward(sender.input_sender(), identity);

        let category_audio_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Audio,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_development_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Development,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_games_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Games,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_graphics_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Graphics,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_web_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Web,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_video_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Video,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_education_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Education,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_science_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Science,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_office_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Office,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_network_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Network,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_system_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::System,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();
        let category_utility_page = CategoryPageModel::builder()
            .launch(CategoryPageInit {
                category: PkgCategory::Utility,
                system_packages_type: syspkgtype.clone(),
            })
            .detach();

        let installed_page = InstalledPageModel::builder()
            .launch(syspkgtype.clone())
            .forward(sender.input_sender(), identity);

        let update_page = UpdatePageModel::builder()
            .launch(UpdatePageInit {
                window: root.clone().upcast(),
                systype: syspkgtype.clone(),
                config: config.clone(),
                online,
            })
            .forward(sender.input_sender(), identity);
        let rebuild = RebuildModel::builder()
            .launch_with_broker(root.clone().upcast(), &REBUILD_BROKER)
            .forward(sender.input_sender(), identity);
        let welcomepage = WelcomeModel::builder()
            .launch(root.clone().into())
            .forward(sender.input_sender(), identity);

        let mut model = AppModel {
            navigation: adw::NavigationSplitView::new(),
            mainwindow: root.clone(),
            config,
            windowloading,
            loaderrordialog,
            busy: true,
            appdata: HashMap::new(),
            installeduserpkgs: HashMap::new(),
            installedsystempkgs: HashSet::new(),
            syspkgtype,
            recommended_apps: Vec::new(),
            category_apps_recommended: HashMap::new(),
            category_apps_all: HashMap::new(),

            search_page,
            explore_page,

            category_audio_page,
            category_development_page,
            category_games_page,
            category_graphics_page,
            category_web_page,
            category_video_page,
            category_education_page,
            category_science_page,
            category_office_page,
            category_network_page,
            category_system_page,
            category_utility_page,

            installed_page,
            update_page,
            package_page: None,
            viewstack: adw::ViewStack::new(),
            installedpagebusy: vec![],
            rebuild,
            welcomepage,
            preferencespage,
            online,
            updates_count: 0,
            tracker: 0,
        };

        {
            let sender = sender.clone();
            adw::StyleManager::default()
                .connect_dark_notify(move |x| sender.input(AppMsg::SetDarkMode(x.is_dark())));
        }

        sender.input(AppMsg::SetDarkMode(adw::StyleManager::default().is_dark()));

        if welcome && nixos {
            model.welcomepage.emit(WelcomeMsg::Show);
        } else {
            model.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                model.syspkgtype.clone(),
                model.config.clone(),
            ));
        }

        let widgets = view_output!();
        model.navigation = widgets.navigation.clone();

        widgets
            .view_stack
            .page(model.installed_page.widget())
            .set_starts_section(true);

        let mut group = RelmActionGroup::<MenuActionGroup>::new();

        let aboutpage: RelmAction<AboutAction> = {
            RelmAction::new_stateless(move |_| {
                AboutPageModel::builder().launch(()).detach();
            })
        };

        let prefernecespage_action: RelmAction<PreferencesAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppMsg::ShowPreferences);
            })
        };

        group.add_action(aboutpage);
        group.add_action(prefernecespage_action);
        let actions = group.into_action_group();
        widgets
            .main_window
            .insert_action_group("menu", Some(&actions));

        widgets.load_window_size();

        // if model.updates_count > 0 {
        //     updatesvs.set_badge_number(
        //         total_updates
        //             .try_into()
        //             .expect("can not convert usize value of total_updates to i32"),
        //     );
        // } else {
        //     page.set_title(Some(&gettext("Updates")));
        // }

        let cli = crate::utils::cli::cli::parse_cli();

        if let Some(scheme) = cli.scheme {
            let sender1 = sender.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                sender1.input(AppMsg::OpenPkgByScheme(scheme));
            });
        }

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.reset();
        match msg {
            AppMsg::TryLoad => {
                self.busy = true;
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                    self.syspkgtype.clone(),
                    self.config.clone(),
                ));
            }
            AppMsg::UpdateDB => {
                self.windowloading
                    .emit(WindowAsyncHandlerMsg::UpdateDB(self.syspkgtype.clone()));
            }
            AppMsg::LoadConfig(config) => {
                info!("AppMsg::LoadConfig");
                *NIX_DATA_CONFIG_STATE.write() = config.clone();
                self.config = config;
                if let Err(e) = editconfig(self.config.clone()) {
                    warn!("Error editing config: {}", e);
                }
                let nixos = Path::new("/etc/nixos").exists();
                self.syspkgtype = if self.config.systemconfig.is_none() || !nixos {
                    SystemPkgs::None
                } else {
                    match fs::read_to_string("/run/current-system/nixos-version") {
                        Ok(s) => {
                            if !Path::new("/nix/var/nix/profiles/per-user/root/channels/nixos")
                                .exists()
                                || self.config.flake.is_some()
                            {
                                SystemPkgs::Flake
                            } else if let Some(last) = s.split('.').next_back() {
                                if last.len() == 7 || last == "dirty" || last == "git" {
                                    SystemPkgs::Flake
                                } else {
                                    SystemPkgs::None
                                }
                            } else {
                                SystemPkgs::None
                            }
                        }
                        Err(_) => SystemPkgs::None,
                    }
                };
                self.update_page
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.update_page
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.windowloading.emit(WindowAsyncHandlerMsg::CheckCache(
                    self.syspkgtype.clone(),
                    self.config.clone(),
                ));
            }
            AppMsg::Close => {
                relm4::main_application().quit();
            }
            AppMsg::LoadError(msg, msg2) => {
                self.busy = false;
                self.loaderrordialog.emit(LoadErrorMsg::Show(msg, msg2));
            }
            AppMsg::ShowPreferences => {
                // Reload config from file before showing preferences
                if let Some(updated_config) = getconfig() {
                    info!("Reloaded config from file: {:?}", updated_config);
                    self.config = updated_config;
                }
                self.preferencespage
                    .emit(PreferencesPageMsg::Show(self.config.clone()));
            }
            AppMsg::UpdateSysconfig(systemconfig) => {
                info!(
                    "AppMsg::UpdateSysconfig - received config: {:?}",
                    systemconfig
                );
                self.config = NixDataConfig {
                    systemconfig: systemconfig.clone(),
                    flake: self.config.flake.clone(),
                    flakearg: self.config.flakearg.clone(),
                    generations: self.config.generations,
                };
                info!("Full config to save: {:?}", self.config);
                if let Err(e) = editconfig(self.config.clone()) {
                    warn!("Failed to update system config: {}", e);
                } else {
                    info!("Successfully saved system config to file");
                }
                let nixos = Path::new("/etc/nixos").exists();
                if systemconfig.is_some() && nixos {
                    if self.syspkgtype == SystemPkgs::None {
                        if self.config.flake.is_some() {
                            self.syspkgtype = SystemPkgs::Flake;
                        } else {
                            self.syspkgtype = SystemPkgs::None;
                        }
                    }
                } else {
                    self.syspkgtype = SystemPkgs::None;
                }

                self.update_page
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.update_page
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.installed_page
                    .emit(InstalledPageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
            }
            AppMsg::UpdateFlake(flake, flakearg) => {
                info!(
                    "AppMsg::UpdateFlake - received flake: {:?}, arg: {:?}",
                    flake, flakearg
                );
                self.config = NixDataConfig {
                    systemconfig: self.config.systemconfig.clone(),
                    flake: flake.clone(),
                    flakearg,
                    generations: self.config.generations,
                };
                info!("Full config to save: {:?}", self.config);
                if let Err(e) = editconfig(self.config.clone()) {
                    warn!("Failed to update flake config: {}", e);
                } else {
                    info!("Successfully saved flake config to file");
                }

                let nixos = Path::new("/etc/nixos").exists();
                if nixos {
                    if flake.is_some() {
                        self.syspkgtype = SystemPkgs::Flake;
                    } else {
                        self.syspkgtype = SystemPkgs::None;
                    }
                }

                self.update_page
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.update_page
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.installed_page
                    .emit(InstalledPageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
            }
            AppMsg::Initialize(
                app_data,
                recommended_apps,
                category_apps_recommended,
                category_apps_all,
            ) => {
                info!("AppMsg::Initialize");
                self.set_appdata(app_data);
                self.set_recommended_apps(recommended_apps);
                self.set_category_apps_recommended(category_apps_recommended);
                self.set_category_apps_all(category_apps_all);

                self.update_page
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));

                sender.input(AppMsg::UpdateInstalledPkgs);

                sender.input(AppMsg::LoadRecommended);

                for category in [
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
                ] {
                    sender.input(AppMsg::LoadCategory(category))
                }

                self.busy = false;
            }
            AppMsg::OpenPkgByScheme(_scheme) => {
                // TODO: rewrite this logic for new structure
                // match scheme {
                //     cli::scheme::Scheme::AppStream { id, alt: _ } => {
                //         let package = self
                //             .appdata
                //             .iter()
                //             .find(|(_, appdata)| appdata.id == id)
                //             .map(|(_, appdata)| appdata.package.clone());
                //         if let Some(package) = package {
                //             sender.input(AppMsg::OpenPkg(package));
                //         } else {
                //             warn!("App could not be found be id: {:?}", package);
                //         }
                //     }
                //     cli::scheme::Scheme::NixPkg(package) => sender.input(AppMsg::OpenPkg(package)),
                // };
            }
            AppMsg::UpdateInstalledPkgs => {
                info!("AppMsg::UpdateInstalledPkgs");
                let systemconfig = self.config.systemconfig.clone();
                let syspkgtype = self.syspkgtype.clone();
                sender.oneshot_command(async move {
                    let installedsystempkgs = if let Some(config) = &systemconfig {
                        match syspkgtype {
                            SystemPkgs::Flake => {
                                let pkgs =
                                    nix_data_xinux::cache::flakes::getflakepkgs(&[config]).await;
                                if let Ok(pkgs) = pkgs {
                                    pkgs.keys().cloned().collect::<HashSet<String>>()
                                } else {
                                    HashSet::new()
                                }
                            }
                            _ => HashSet::new(),
                        }
                    } else {
                        HashSet::new()
                    };

                    let installeduserpkgs = {
                        let pkgs = nix_data_xinux::cache::profile::getprofilepkgs_versioned().await;
                        if let Ok(pkgs) = pkgs {
                            pkgs
                        } else {
                            warn!("this is errrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr");
                            HashMap::new()
                        }
                    };
                    AppAsyncMsg::UpdateInstalledPkgs(installedsystempkgs, installeduserpkgs)
                });
            }
            AppMsg::UpdateInstalledPage => {
                info!("AppMsg::UpdateInstalledPage");
                let mut installeduseritems = vec![];
                let mut updateuseritems = vec![];
                // let pool = SqlitePool::connect(&self.pkgdb).await.unwrap();
                debug!("Installed user pkgs: {:?}", self.installeduserpkgs);
                debug!("Installed system pkgs: {:?}", self.installedsystempkgs);
                let package_db = &PACKAGES_DB_STATE.read().packages_db;
                if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", package_db)).await {
                    warn!(
                        "UserPkgs::Profile is installeduserpkgs len: {:?}",
                        self.installeduserpkgs.len()
                    );

                    for installedpkg in self.installeduserpkgs.keys() {
                        debug!("Checking package {}", installedpkg);
                        let (pname, version): (String, String) =
                            sqlx::query_as("SELECT pname, version FROM pkgs WHERE attribute = $1")
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await
                                .unwrap();
                        let (description,): (String,) =
                            sqlx::query_as("SELECT description FROM meta WHERE attribute = $1")
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await
                                .unwrap();
                        let mut name = pname.to_string();
                        let mut summary = if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        };
                        let mut icon = None;
                        if let Some(data) = self.appdata.get(installedpkg) {
                            if let Some(n) = &data.name
                                && let Some(n) = n.get("C")
                            {
                                name = n.to_string();
                            }
                            if let Some(s) = &data.summary
                                && let Some(s) = s.get("C")
                            {
                                summary = Some(s.to_string());
                            }
                            if let Some(i) = &data.icon
                                && let Some(i) = &i.cached
                            {
                                icon = Some(i[0].name.clone());
                            }
                        }
                        installeduseritems.push(InstalledItem {
                            name: name.to_string(),
                            pname: pname.to_string(),
                            pkg: installedpkg.clone(),
                            summary: summary.clone(),
                            icon: icon.clone(),
                            pkgtype: InstallType::User,
                            busy: self
                                .installedpagebusy
                                .contains(&(installedpkg.clone(), InstallType::User)),
                            version: version.to_string(),
                        });
                        let nixpkgs_db = &PACKAGES_DB_STATE.read().nixpkgs_db;
                        if let Some(latest) = &nixpkgs_db
                            && let Ok(latestpool) =
                                &SqlitePool::connect(&format!("sqlite://{}", latest)).await
                        {
                            let (newver,): (String,) =
                                sqlx::query_as("SELECT version FROM pkgs WHERE attribute = $1")
                                    .bind(installedpkg)
                                    .fetch_one(latestpool)
                                    .await
                                    .unwrap();
                            debug!("PROFILE: {} {} {}", installedpkg, version, newver);
                            if version != newver {
                                updateuseritems.push(UpdateItem {
                                    name,
                                    pname,
                                    pkg: Some(installedpkg.clone()),
                                    summary,
                                    icon,
                                    pkgtype: InstallType::System,
                                    verfrom: Some(version.clone()),
                                    verto: Some(newver.clone()),
                                })
                            }
                        }
                    }

                    warn!("installeduseritems: {:?}", installeduseritems);
                    installeduseritems.sort_by_key(|a| a.name.to_lowercase());
                    let mut installedsystemitems = vec![];
                    let mut updatesystemitems = vec![];
                    for installedpkg in &self.installedsystempkgs {
                        let versionpname: sqlx::Result<(String, String)> =
                            sqlx::query_as("SELECT pname, version FROM pkgs where attribute = $1")
                                .bind(installedpkg)
                                .fetch_one(pool)
                                .await;
                        if let Ok((pname, version)) = versionpname {
                            let desc: sqlx::Result<(String,)> =
                                sqlx::query_as("SELECT description FROM meta WHERE attribute = $1")
                                    .bind(installedpkg)
                                    .fetch_one(pool)
                                    .await;
                            if let Ok((description,)) = desc {
                                let mut name = pname.to_string();
                                let mut summary = if description.is_empty() {
                                    None
                                } else {
                                    Some(description)
                                };
                                let mut icon = None;
                                if let Some(data) = self.appdata.get(installedpkg) {
                                    if let Some(n) = &data.name
                                        && let Some(n) = n.get("C")
                                    {
                                        name = n.to_string();
                                    }
                                    if let Some(s) = &data.summary
                                        && let Some(s) = s.get("C")
                                    {
                                        summary = Some(s.to_string());
                                    }
                                    if let Some(i) = &data.icon
                                        && let Some(i) = &i.cached
                                    {
                                        icon = Some(i[0].name.clone());
                                    }
                                }
                                installedsystemitems.push(InstalledItem {
                                    name: name.to_string(),
                                    pname: pname.to_string(),
                                    pkg: installedpkg.clone(),
                                    summary: summary.clone(),
                                    icon: icon.clone(),
                                    pkgtype: InstallType::System,
                                    busy: self
                                        .installedpagebusy
                                        .contains(&(installedpkg.clone(), InstallType::System)),
                                    version: version.to_string(),
                                });
                                let system_db = &PACKAGES_DB_STATE.read().system_db;
                                if let Some(current) = &system_db
                                    && let Ok(currentpool) =
                                        &SqlitePool::connect(&format!("sqlite://{}", current)).await
                                {
                                    let (currver,): (String,) = sqlx::query_as(
                                        "SELECT version FROM pkgs WHERE attribute = $1",
                                    )
                                    .bind(installedpkg)
                                    .fetch_one(currentpool)
                                    .await
                                    .unwrap();
                                    debug!("SYSTEM: {} {} {}", installedpkg, currver, version);
                                    if version != currver {
                                        updatesystemitems.push(UpdateItem {
                                            name,
                                            pname,
                                            pkg: Some(installedpkg.clone()),
                                            summary,
                                            icon,
                                            pkgtype: InstallType::System,
                                            verfrom: Some(currver.clone()),
                                            verto: Some(version.clone()),
                                        })
                                    }
                                }
                            }
                        }
                    }

                    // Add NixOS system to update list
                    match self.syspkgtype {
                        SystemPkgs::Flake => {
                            if let Ok(Some((old, new))) = nix_data_xinux::cache::flakes::uptodate()
                            {
                                println!("old flake ver: {old:?}");
                                println!("new flake ver: {new:?}");
                                updatesystemitems.insert(
                                    0,
                                    UpdateItem {
                                        name: gettext("NixOS System"),
                                        pname: String::new(),
                                        pkg: None,
                                        summary: Some(gettext(
                                            "NixOS internal packages and modules",
                                        )),
                                        icon: None,
                                        pkgtype: InstallType::System,
                                        verfrom: Some(old),
                                        verto: Some(new),
                                    },
                                )
                            }
                        }
                        SystemPkgs::None => {}
                    }

                    installedsystemitems.sort_by_key(|a| a.name.to_lowercase());
                    let total_updates = updateuseritems.len() + updatesystemitems.len();
                    // let total_updates = 5; // testing
                    self.set_updates_count(total_updates);

                    // Update the updates page tab title with badge
                    if let Some(updates_child) = self.viewstack.child_by_name("updates") {
                        let page = self.viewstack.page(&updates_child);
                        if total_updates > 0 {
                            page.set_badge_number(total_updates.try_into().unwrap_or_default());
                            page.set_needs_attention(true);
                        } else {
                            page.set_title(Some(&gettext("Updates")));
                            page.set_badge_number(0);
                            page.set_needs_attention(false);
                        }
                    }

                    *INSTALLED_PACKAGES_STATE.write() = InstalledPackagesState {
                        installed_system_packages: installedsystemitems,
                        installed_user_packages: installeduseritems,
                    };

                    self.update_page
                        .emit(UpdatePageMsg::Update(updateuseritems, updatesystemitems));
                } else {
                    error!("Could not connect to pkgdb");
                }
            }
            AppMsg::AddInstalledToWorkQueue(work) => {
                let p = match work.install_type {
                    InstallType::User => work.package_name.to_string(),
                    InstallType::System => work.package.to_string(),
                };
                self.installedpagebusy.push((p, work.install_type.clone()));
            }
            AppMsg::RemoveInstalledBusy(work) => {
                let p = match work.install_type {
                    InstallType::User => work.package_name.to_string(),
                    InstallType::System => work.package.to_string(),
                };
                self.installedpagebusy
                    .retain(|(x, y)| x != &p && y != &work.install_type);
                self.installed_page.emit(InstalledPageMsg::UnsetBusy(work));
            }
            AppMsg::LoadRecommended => {
                let packages_db = PACKAGES_DB_STATE.read().packages_db.clone();

                let recommended_apps = self.recommended_apps.clone();

                let app_data = self.appdata.clone();
                let installed_user = self.installeduserpkgs.clone();
                let installed_system = self.installedsystempkgs.clone();

                sender.oneshot_command(async move {
                    let mut package_tiles = vec![];

                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{packages_db}")).await
                    {
                        for package in recommended_apps {
                            // TODO: unify CategoryTile and PkgTile structs or improve this logic
                            let category_tile = make_category_tile(
                                package.clone(),
                                &app_data,
                                pool,
                                installed_user.contains_key(&package),
                                installed_system.contains(&package),
                            )
                            .await;
                            let package_tile = PkgTile {
                                name: category_tile.name,
                                pkg: category_tile.package,
                                pname: category_tile.package_name,
                                summary: category_tile.summary,
                                icon: category_tile.icon,
                                installeduser: category_tile.installed_user,
                                installedsystem: category_tile.installed_system,
                            };

                            package_tiles.push(package_tile);
                        }
                    } else {
                        error!("Failed to connect to pkgdb");
                    }
                    AppAsyncMsg::LoadRecommended(package_tiles)
                });
            }
            AppMsg::LoadCategory(category) => {
                info!("AppMsg::LoadCategory({:?})", category);

                let packages_db = PACKAGES_DB_STATE.read().packages_db.clone();

                let category_apps_recommended = self
                    .category_apps_recommended
                    .get(&category)
                    .map_or_else(std::vec::Vec::new, std::clone::Clone::clone);
                let category_apps_all = self
                    .category_apps_all
                    .get(&category)
                    .map_or_else(std::vec::Vec::new, std::clone::Clone::clone);

                let app_data = self.appdata.clone();
                let installed_user = self.installeduserpkgs.clone();
                let installed_system = self.installedsystempkgs.clone();

                sender.oneshot_command(async move {
                    let mut catrec = vec![];
                    let mut catall = vec![];

                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{packages_db}")).await
                    {
                        for package in category_apps_recommended {
                            catrec.push(
                                make_category_tile(
                                    package.clone(),
                                    &app_data,
                                    pool,
                                    installed_user.contains_key(&package),
                                    installed_system.contains(&package),
                                )
                                .await,
                            );
                        }
                        for package in category_apps_all {
                            catall.push(
                                make_category_tile(
                                    package.clone(),
                                    &app_data,
                                    pool,
                                    installed_user.contains_key(&package),
                                    installed_system.contains(&package),
                                )
                                .await,
                            );
                        }
                    } else {
                        error!("Failed to connect to pkgdb");
                    }
                    AppAsyncMsg::LoadCategory(category, catrec, catall)
                });
            }
            AppMsg::SetDarkMode(dark) => {
                info!("AppMsg::SetDarkMode({})", dark);
                let scheme = if dark { "Adwaita-dark" } else { "Adwaita" };
                self.rebuild.emit(RebuildMsg::SetScheme(scheme.to_string()));
            }
            AppMsg::GetUnavailableItems(userpkgs, syspkgs, updatetype) => {
                info!("AppMsg::GetUnavailableItems");
                let appdata: HashMap<String, AppData> = self
                    .appdata
                    .iter()
                    .filter_map(|(k, v)| {
                        if syspkgs.contains_key(k) || userpkgs.contains_key(k) {
                            Some((k.to_string(), v.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let poolref = PACKAGES_DB_STATE.read().packages_db.clone();
                relm4::spawn(async move {
                    let mut unavailableuser = vec![];
                    let mut unavailablesys = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", poolref)).await {
                        let mut sortuserpkgs = userpkgs.into_iter().collect::<Vec<_>>();
                        sortuserpkgs.sort();
                        for (pkg, msg) in sortuserpkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: Result<(String,), sqlx::Error> =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await;
                                if let Ok(pname) = pname {
                                    unavailableuser.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pname.0).to_string()
                                        } else {
                                            pname.0.to_string()
                                        },
                                        pname: pname.0.to_string(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                } else {
                                    unavailableuser.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pkg).to_string()
                                        } else {
                                            pkg.to_string()
                                        },
                                        pname: String::new(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                }
                            } else {
                                unavailableuser.push(UnavailableItemModel {
                                    pkg: pkg.to_string(),
                                    name: pkg.to_string(),
                                    pname: String::new(),
                                    icon: None,
                                    message: msg,
                                })
                            }
                        }
                        let mut sortsyspkgs = syspkgs.into_iter().collect::<Vec<_>>();
                        sortsyspkgs.sort();
                        for (pkg, msg) in sortsyspkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: Result<(String,), sqlx::Error> =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await;
                                if let Ok(pname) = pname {
                                    unavailablesys.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pname.0).to_string()
                                        } else {
                                            pname.0.to_string()
                                        },
                                        pname: pname.0.to_string(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                } else {
                                    unavailablesys.push(UnavailableItemModel {
                                        pkg: pkg.to_string(),
                                        name: if let Some(name) = &data.name {
                                            name.get("C").unwrap_or(&pkg).to_string()
                                        } else {
                                            pkg.to_string()
                                        },
                                        pname: String::new(),
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        message: msg,
                                    })
                                }
                            } else {
                                unavailablesys.push(UnavailableItemModel {
                                    pkg: pkg.to_string(),
                                    name: pkg.to_string(),
                                    pname: String::new(),
                                    icon: None,
                                    message: msg,
                                })
                            }
                        }
                    }
                    UNAVAILABLE_BROKER.send(UnavailableDialogMsg::Show(
                        unavailableuser,
                        unavailablesys,
                        updatetype,
                    ));
                });
            }
            AppMsg::CheckNetwork => {
                let selfonline = self.online;
                let senderclone = sender.clone();
                sender.oneshot_command(async move {
                    info!("AppMsg::CheckNetwork");
                    let online = checkonline_async().await;
                    if online && !selfonline {
                        senderclone.input(AppMsg::UpdateDB);
                    }
                    AppAsyncMsg::SetNetwork(online)
                });
            }
            AppMsg::Noop => {}
        }
    }

    async fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppAsyncMsg::UpdateInstalledPkgs(installedsystempkgs, installeduserpkgs) => {
                // TODO: maybe create macro to update installed pkgs
                info!("AppAsyncMsg::UpdateInstalledPkgs");
                if installedsystempkgs != self.installedsystempkgs
                    || installeduserpkgs != self.installeduserpkgs
                {
                    warn!("Changes needed!");
                    self.installedsystempkgs = installedsystempkgs.clone();
                    self.installeduserpkgs = installeduserpkgs.clone();
                }
                // Always refresh the update page
                sender.input(AppMsg::UpdateInstalledPage);
                info!("DONE AppAsyncMsg::UpdateInstalledPkgs");
            }
            AppAsyncMsg::LoadRecommended(recommended_apps) => {
                self.explore_page
                    .emit(ExplorePageMsg::UpdateRecommendedPackages(recommended_apps));
            }
            AppAsyncMsg::LoadCategory(category, recommended_apps, all_apps) => {
                match category {
                    PkgCategory::Audio => &self.category_audio_page,
                    PkgCategory::Development => &self.category_development_page,
                    PkgCategory::Games => &self.category_games_page,
                    PkgCategory::Graphics => &self.category_graphics_page,
                    PkgCategory::Web => &self.category_web_page,
                    PkgCategory::Video => &self.category_video_page,
                    PkgCategory::Education => &self.category_education_page,
                    PkgCategory::Science => &self.category_science_page,
                    PkgCategory::Office => &self.category_office_page,
                    PkgCategory::Network => &self.category_network_page,
                    PkgCategory::System => &self.category_system_page,
                    PkgCategory::Utility => &self.category_utility_page,
                }
                .sender()
                .emit(CategoryPageMsg::UpdatePackages(recommended_apps, all_apps));
            }
            AppAsyncMsg::SetNetwork(online) => {
                self.online = online;
                self.update_page.emit(UpdatePageMsg::UpdateOnline(online));
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        widgets.save_window_size().ok();
    }
}

impl AppWidgets {
    fn save_window_size(&self) -> Result<(), gtk::glib::BoolError> {
        let settings = gtk::gio::Settings::new(config::APP_ID);
        let (width, height) = self.main_window.default_size();

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;

        settings.set_boolean("is-maximized", self.main_window.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = gtk::gio::Settings::new(config::APP_ID);

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.main_window.set_default_size(width, height);

        if is_maximized {
            self.main_window.maximize();
        }
    }
}

async fn make_category_tile(
    package: String,
    app_data: &HashMap<String, AppData>,
    pool: &sqlx::Pool<Sqlite>,
    installed_user: bool,
    installed_system: bool,
) -> CategoryTile {
    if let Some(data) = app_data.get(&package) {
        let (package_name,): (String,) =
            sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                .bind(&package)
                .fetch_one(pool)
                .await
                .unwrap_or_default();

        let name = data
            .name
            .as_ref()
            .and_then(|names| names.get("C"))
            .map_or_else(|| package_name.clone(), std::clone::Clone::clone);

        let icon = data
            .icon
            .as_ref()
            .and_then(|icons| icons.cached.as_ref())
            .and_then(|icon| icon.first())
            .map(|icon| icon.name.clone());

        let summary = data
            .summary
            .as_ref()
            .and_then(|summaries| summaries.get("C"))
            .map_or_else(String::default, std::clone::Clone::clone);

        CategoryTile {
            name,
            package,
            package_name,
            summary,
            icon,
            installed_user,
            installed_system,
        }
    } else {
        let (package_name, summary): (String, String) = sqlx::query_as("SELECT pname, description FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1").bind(&package).fetch_one(pool).await.unwrap();

        CategoryTile {
            name: package_name.clone(),
            package,
            package_name,
            summary,
            icon: None,
            installed_user,
            installed_system,
        }
    }
}

relm4::new_action_group!(MenuActionGroup, "menu");
relm4::new_stateless_action!(AboutAction, MenuActionGroup, "about");
relm4::new_stateless_action!(PreferencesAction, MenuActionGroup, "preferences");
