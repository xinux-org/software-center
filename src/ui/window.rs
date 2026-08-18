use gettextrs::gettext;
use log::*;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{
    self, AsyncComponentSender, Component, ComponentController, Controller, MessageBroker, Sender,
    WorkerController,
    actions::{RelmAction, RelmActionGroup},
    adw::{self, prelude::*},
    gtk::{self},
    prelude::{AsyncComponent, AsyncComponentParts},
};
use spdx::Expression;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    fs,
    path::Path,
    time::Duration,
};

use crate::{
    APPINFO, config,
    ui::{
        about::about_page::AboutPageModel,
        category::{
            category_page::{CategoryPageModel, CategoryPageMsg},
            components::{categories::PkgCategory, category_tile::CategoryTile},
        },
        explore::explore_page::{ExplorePageModel, ExplorePageMsg},
        installed::{
            components::installed_item::InstalledItem,
            installed_page::{InstalledPageModel, InstalledPageMsg},
        },
        package::{
            components::package_tile::PkgTile,
            package_page::{
                InstallType, License, PkgInitModel, PkgModel, PkgMsg, PkgPageInit, WorkPkg,
            },
        },
        preferences::preferences_page::{PreferencesPageModel, PreferencesPageMsg},
        rebuild::rebuild_model::{RebuildModel, RebuildMsg},
        search::{
            components::search_item::SearchItem,
            search_page::{SearchPageModel, SearchPageMsg},
        },
        update::{
            components::update_item::UpdateItem,
            unavailable_dialog::{UnavailableDialogMsg, UnavailableItemModel},
            update_page::{
                UNAVAILABLE_BROKER, UpdatePageInit, UpdatePageModel, UpdatePageMsg, UpdateType,
            },
        },
        welcome::welcome_page::{WelcomeModel, WelcomeMsg},
        windowloading::{LoadErrorModel, LoadErrorMsg, WindowAsyncHandler, WindowAsyncHandlerMsg},
    },
    utils::{
        cli,
        config::{editconfig, getconfig},
        online::{checkonline, checkonline_async},
        packages::{AppData, LicenseEnum, PkgMaintainer, Platform},
    },
};

pub static REBUILD_BROKER: MessageBroker<RebuildMsg> = MessageBroker::new();

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SystemPkgs {
    Flake,
    None,
}

#[tracker::track]
pub struct AppModel {
    navigation: adw::NavigationView,
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
    #[tracker::no_eq]
    pkgdb: String,
    #[tracker::no_eq]
    nixpkgsdb: Option<String>,
    #[tracker::no_eq]
    systemdb: Option<String>,
    appdata: HashMap<String, AppData>,
    installeduserpkgs: HashMap<String, String>,
    installedsystempkgs: HashSet<String>,
    syspkgtype: SystemPkgs,
    categoryrec: HashMap<PkgCategory, Vec<String>>,
    categoryall: HashMap<PkgCategory, Vec<String>>,
    #[tracker::no_eq]
    pkgpage: Controller<PkgModel>,
    #[tracker::no_eq]
    searchpage: Controller<SearchPageModel>,
    #[tracker::no_eq]
    categorypage: Controller<CategoryPageModel>,
    searching: bool,
    searchquery: String,
    vschild: String,
    showvsbar: bool,
    #[tracker::no_eq]
    preferencespage: Controller<PreferencesPageModel>,
    #[tracker::no_eq]
    explore_page: Controller<ExplorePageModel>,
    #[tracker::no_eq]
    installedpage: Controller<InstalledPageModel>,
    #[tracker::no_eq]
    updatepage: Controller<UpdatePageModel>,
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
        String,
        Option<String>,
        Option<String>,
        HashMap<String, AppData>,
        Vec<String>,
        // rec apps based on different category below 5 vectors
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        HashMap<PkgCategory, Vec<String>>,
        HashMap<PkgCategory, Vec<String>>,
    ),
    OpenPkgByScheme(cli::scheme::Scheme),
    OpenPkg(String),
    // UpdatePkgs(Option<Vec<String>>),
    UpdateInstalledPkgs,
    UpdateInstalledPage,
    // UpdateUpdatePkgs,
    UpdateCategoryPkgs,
    SetSearch(bool),
    SetVsBar(bool),
    Search(String),
    AddInstalledToWorkQueue(WorkPkg),
    RemoveInstalledBusy(WorkPkg),
    OpenCategoryPage(PkgCategory),
    LoadCategory(PkgCategory),
    UpdateRecPkgs(Vec<String>, Option<PkgCategory>), // if None then itʻs recomended apps
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
    Search(String, Vec<SearchItem>),
    UpdateRecPkgs(Vec<PkgTile>, Option<PkgCategory>),
    UpdateInstalledPkgs(HashSet<String>, HashMap<String, String>),
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

            // desktop mode
            add_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
                adw::BreakpointConditionLengthType::MinWidth,
                610.0,
                adw::LengthUnit::Sp,
            )) {
                add_setter: (&switcher_title, "policy", Some(&adw::ViewSwitcherPolicy::Wide.into())),
            },

            // tablet mode
            add_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
                adw::BreakpointConditionLengthType::MaxWidth,
                600.0,
                adw::LengthUnit::Sp,
            )) {
                add_setter: (&switcher_title, "policy", Some(&adw::ViewSwitcherPolicy::Narrow.into())),
            },

            // mobile mode
            add_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
                adw::BreakpointConditionLengthType::MaxWidth,
                500.0,
                adw::LengthUnit::Sp,
            )) {
                add_setter: (&header_bar, "show-title", Some(&false.into())),
                add_setter: (&switcher_bar, "reveal", Some(&true.into())),
            },

            // FIXME: use more idiomatic gtk::Stack to switch pages.
            // see example: https://git.oss.uzinfocom.uz/xinux/settings/src/branch/main/src/ui/wifi/wifi_panel.rs#L141-L142
            #[transition(Crossfade)]
            #[name(main_stack)]
            if model.busy {
                gtk::Box {
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    set_orientation: gtk::Orientation::Vertical,
                    adw::HeaderBar {
                        add_css_class: "flat",
                        #[wrap(Some)]
                        set_title_widget = &gtk::Label {
                            set_label: &gettext("Nix Software Center")
                        }
                    },
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
                    }
                }
            } else {
                #[name = "navigation"]
                adw::NavigationView {
                    add = &adw::NavigationPage {
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            #[name(header_bar)]
                            adw::HeaderBar {
                                pack_start: searchbtn = &gtk::ToggleButton {
                                    add_css_class: "flat",
                                    set_icon_name: "system-search-symbolic",
                                    #[watch]
                                    #[block_signal(searchtoggle)]
                                    set_active: model.searching,
                                    connect_toggled[sender] => move |x| {
                                        sender.input(AppMsg::SetSearch(x.is_active()))
                                    } @searchtoggle

                                },

                                #[name(switcher_title)]
                                #[wrap(Some)]
                                set_title_widget = &adw::ViewSwitcher {
                                    set_stack: Some(viewstack),
                                    #[watch] // when we do wider
                                    set_policy: adw::ViewSwitcherPolicy::Wide,
                                },

                                pack_end: menu = &gtk::MenuButton {
                                    add_css_class: "flat",
                                    set_icon_name: "open-menu-symbolic",
                                    #[wrap(Some)]
                                    set_popover = &gtk::PopoverMenu::from_model(Some(&mainmenu)) {
                                        add_css_class: "menu"
                                    }
                                }
                            },
                            gtk::SearchBar {
                                #[watch]
                                set_search_mode: model.searching,
                                #[wrap(Some)]
                                set_child = &adw::Clamp {
                                    set_hexpand: true,
                                    gtk::SearchEntry {
                                        #[track(model.changed(AppModel::searching()) && model.searching)]
                                        grab_focus: (),
                                        #[track(model.changed(AppModel::searching()) && !model.searching)]
                                        set_text: "",
                                        connect_search_changed[sender] => move |x| {
                                            if x.text().len() > 1 {
                                                sender.input(AppMsg::Search(x.text().to_string()))
                                            }
                                        }
                                    }
                                }
                            },
                            #[local_ref]
                            viewstack -> adw::ViewStack {
                                add: model.explore_page.widget(),
                                add: model.installedpage.widget(),
                                add: model.searchpage.widget(),
                                add: model.updatepage.widget(),
                            },

                            #[name(switcher_bar)]
                            adw::ViewSwitcherBar {
                                set_stack: Some(viewstack),
                            }
                        },
                    },
                }
            }
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
        let pkgpage = PkgModel::builder()
            .launch(PkgPageInit {
                syspkgs: syspkgtype.clone(),
                config: config.clone(),
                online,
            })
            .forward(sender.input_sender(), identity);
        let searchpage = SearchPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let categorypage = CategoryPageModel::builder()
            .launch(())
            .forward(sender.input_sender(), identity);
        let explore_page = ExplorePageModel::builder()
            .launch(syspkgtype.clone())
            .forward(sender.input_sender(), identity);
        let installedpage = InstalledPageModel::builder()
            .launch(syspkgtype.clone())
            .forward(sender.input_sender(), identity);
        let updatepage = UpdatePageModel::builder()
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
            navigation: adw::NavigationView::new(),
            mainwindow: root.clone(),
            config,
            windowloading,
            loaderrordialog,
            busy: true,
            pkgdb: String::new(),
            nixpkgsdb: None,
            systemdb: None,
            appdata: HashMap::new(),
            installeduserpkgs: HashMap::new(),
            installedsystempkgs: HashSet::new(),
            syspkgtype,
            categoryrec: HashMap::new(),
            categoryall: HashMap::new(),
            pkgpage,
            searchpage,
            categorypage,
            searching: false,
            searchquery: String::default(),
            vschild: String::default(),
            showvsbar: false,
            explore_page,
            installedpage,
            updatepage,
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
        let viewstack = &model.viewstack;

        let widgets = view_output!();
        model.navigation = widgets.navigation.clone();

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

        widgets.main_stack.set_vhomogeneous(false);
        widgets.main_stack.set_hhomogeneous(false);
        widgets.load_window_size();
        let frontvs = widgets.viewstack.page(model.explore_page.widget());
        let installedvs = widgets.viewstack.page(model.installedpage.widget());
        let updatesvs = widgets.viewstack.page(model.updatepage.widget());
        let searchvs = widgets.viewstack.page(model.searchpage.widget());
        frontvs.set_title(Some(&gettext("Explore")));
        installedvs.set_title(Some(&gettext("Installed")));
        updatesvs.set_title(Some(&gettext("Updates")));
        frontvs.set_name(Some("explore"));
        installedvs.set_name(Some("installed"));
        searchvs.set_name(Some("search"));
        updatesvs.set_name(Some("updates"));
        frontvs.set_icon_name(Some("nsc-home-symbolic"));
        installedvs.set_icon_name(Some("nsc-installed-symbolic"));
        updatesvs.set_icon_name(Some("nsc-update-symbolic"));

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
                self.pkgpage
                    .emit(PkgMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.updatepage
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

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.pkgpage
                    .emit(PkgMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.installedpage
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

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));
                self.pkgpage
                    .emit(PkgMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
                self.installedpage
                    .emit(InstalledPageMsg::UpdatePkgTypes(self.syspkgtype.clone()));
            }
            AppMsg::Initialize(
                pkgdb,
                nixpkgsdb,
                systemdb,
                appdata,
                recommendedapps,
                devapps,
                gameapps,
                graphickapps,
                webapps,
                videoapps,
                categoryrec,
                categoryall,
            ) => {
                info!("AppMsg::Initialize");
                self.pkgdb = pkgdb;
                self.nixpkgsdb = nixpkgsdb;
                self.systemdb = systemdb;
                self.appdata = appdata;
                self.categoryrec = categoryrec;
                self.categoryall = categoryall;

                self.pkgpage.emit(PkgMsg::UpdateConfig(self.config.clone()));
                self.updatepage
                    .emit(UpdatePageMsg::UpdateConfig(self.config.clone()));

                // TODO: Refactor this in the future
                println!("recommendedapps\n\n\n\n\n\n\n: {:?}", &recommendedapps);
                sender.input(AppMsg::UpdateRecPkgs(recommendedapps, None));
                println!("devapps\n\n\n\n\n\n\n: {:?}", &devapps);
                sender.input(AppMsg::UpdateRecPkgs(
                    devapps,
                    Some(PkgCategory::Development),
                ));
                sender.input(AppMsg::UpdateRecPkgs(gameapps, Some(PkgCategory::Games)));
                sender.input(AppMsg::UpdateRecPkgs(
                    graphickapps,
                    Some(PkgCategory::Graphics),
                ));
                sender.input(AppMsg::UpdateRecPkgs(webapps, Some(PkgCategory::Web)));
                sender.input(AppMsg::UpdateRecPkgs(videoapps, Some(PkgCategory::Video)));

                self.busy = false;
            }
            AppMsg::UpdateRecPkgs(pkgs, pkgs_category) => {
                info!("AppMsg::UpdateRecPkgs");
                let appdata: HashMap<String, AppData> = self
                    .appdata
                    .iter()
                    .filter_map(|(k, v)| {
                        if pkgs.contains(k) {
                            Some((k.to_string(), v.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                let installeduser = self.installeduserpkgs.clone();
                let installedsystem = self.installedsystempkgs.clone();
                let poolref = self.pkgdb.clone();
                sender.oneshot_command(async move {
                    let mut pkgtiles = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", poolref)).await {
                        for pkg in pkgs {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                    sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                        .bind(&pkg)
                                        .fetch_one(pool)
                                        .await
                                        .unwrap();
                                pkgtiles.push(PkgTile {
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
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string())
                                        .unwrap_or_default(),
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                    }
                    AppAsyncMsg::UpdateRecPkgs(pkgtiles, pkgs_category)
                });
            }
            AppMsg::OpenPkgByScheme(scheme) => match scheme {
                cli::scheme::Scheme::AppStream { id, alt: _ } => {
                    let package = self
                        .appdata
                        .iter()
                        .find(|(_, appdata)| appdata.id == id)
                        .map(|(_, appdata)| appdata.package.clone());
                    if let Some(package) = package {
                        sender.input(AppMsg::OpenPkg(package));
                    } else {
                        warn!("App could not be found be id: {:?}", package);
                    }
                }
                cli::scheme::Scheme::NixPkg(package) => sender.input(AppMsg::OpenPkg(package)),
            },
            AppMsg::OpenPkg(pkg) => {
                info!("AppMsg::OpenPkg {}", pkg);
                sender.input(AppMsg::CheckNetwork);
                if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", self.pkgdb)).await {
                    let pkgdata: Result<
                        (
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            String,
                            bool,
                            bool,
                            bool,
                            bool,
                        ),
                        _,
                    > = sqlx::query_as(
                        r#"
SELECT pname, version, system, description, longdescription, license, platforms, maintainers, position, broken, insecure, unsupported, unfree
FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1
                    "#,
                    )
                    .bind(&pkg)
                    .fetch_one(pool)
                    .await;

                    if let Ok((
                        pname,
                        version,
                        system,
                        description,
                        longdescription,
                        licensejson,
                        platformsjson,
                        maintainersjson,
                        position,
                        broken,
                        insecure,
                        unsupported,
                        unfree,
                    )) = pkgdata
                    {
                        let mut name = pname.to_string();
                        let mut summary = if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        };
                        let mut description = if longdescription.is_empty() {
                            None
                        } else {
                            Some(longdescription)
                        };
                        let mut icon = None;
                        let mut screenshots = vec![];
                        let mut licenses = vec![];
                        let mut platforms = vec![];
                        let mut maintainers = vec![];
                        let mut launchable = None;
                        let mut url = None;

                        let app_data = self.appdata.get(&pkg);

                        if let Some(data) = app_data {
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
                            if let Some(d) = &data.description
                                && let Some(d) = d.get("C")
                            {
                                description = Some(d.to_string());
                            }
                            if let Some(i) = &data.icon
                                && let Some(mut i) = i.cached.clone()
                            {
                                i.sort_by_key(|x| x.height);
                                if let Some(i) = i.last() {
                                    icon = Some(format!(
                                        "{}/icons/nixos/{}x{}/{}",
                                        APPINFO, i.width, i.height, i.name
                                    ));
                                }
                            }
                            if let Some(s) = &data.screenshots {
                                for s in s {
                                    if let Some(u) = &s.sourceimage {
                                        if !screenshots.contains(&u.url) {
                                            if s.default == Some(true) {
                                                screenshots.insert(0, u.url.clone());
                                            } else {
                                                screenshots.push(u.url.clone());
                                            }
                                        } else if s.default == Some(true)
                                            && let Some(index) =
                                                screenshots.iter().position(|x| *x == u.url)
                                        {
                                            screenshots.remove(index);
                                            screenshots.insert(0, u.url.clone());
                                        }
                                    }
                                }
                            }
                            if let Some(l) = &data.launchable
                                && let Some(d) = l.desktopid.first()
                            {
                                launchable = Some(d.to_string());
                            }
                            url = data.url.clone();
                        }

                        fn addlicense(pkglicense: &LicenseEnum, licenses: &mut Vec<License>) {
                            match pkglicense {
                                LicenseEnum::Single(l) => {
                                    if let Some(n) = &l.fullname {
                                        let parsed = if let Some(id) = &l.spdxid {
                                            if let Ok(Some(license)) =
                                                Expression::parse(id).map(|p| {
                                                    p.requirements()
                                                        .map(|er| er.req.license.id())
                                                        .collect::<Vec<_>>()[0]
                                                })
                                            {
                                                Some(license)
                                            } else {
                                                None
                                            }
                                        } else if let Ok(Some(license)) =
                                            Expression::parse(n).map(|p| {
                                                p.requirements()
                                                    .map(|er| er.req.license.id())
                                                    .collect::<Vec<_>>()[0]
                                            })
                                        {
                                            Some(license)
                                        } else {
                                            None
                                        };
                                        licenses.push(License {
                                            free: if let Some(f) = l.free {
                                                Some(f)
                                            } else {
                                                parsed.map(|p| {
                                                    p.is_osi_approved() || p.is_fsf_free_libre()
                                                })
                                            },
                                            fullname: n.to_string(),
                                            spdxid: l.spdxid.clone(),
                                            url: if let Some(u) = &l.url {
                                                Some(u.to_string())
                                            } else {
                                                parsed.map(|p| {
                                                    format!(
                                                        "https://spdx.org/licenses/{}.html",
                                                        p.name
                                                    )
                                                })
                                            },
                                        })
                                    } else if let Some(s) = &l.spdxid
                                        && let Ok(Some(license)) = Expression::parse(s).map(|p| {
                                            p.requirements()
                                                .map(|er| er.req.license.id())
                                                .collect::<Vec<_>>()[0]
                                        })
                                    {
                                        licenses.push(License {
                                            free: Some(
                                                license.is_osi_approved()
                                                    || license.is_fsf_free_libre()
                                                    || l.free.unwrap_or(false),
                                            ),
                                            fullname: license.full_name.to_string(),
                                            spdxid: Some(license.name.to_string()),
                                            url: if l.url.is_some() {
                                                l.url.clone()
                                            } else {
                                                Some(format!(
                                                    "https://spdx.org/licenses/{}.html",
                                                    license.name
                                                ))
                                            },
                                        })
                                    }
                                }
                                LicenseEnum::List(lst) => {
                                    for l in lst {
                                        addlicense(&LicenseEnum::Single(l.clone()), licenses);
                                    }
                                }
                                LicenseEnum::SingleStr(s) => {
                                    if let Ok(Some(license)) = Expression::parse(s).map(|p| {
                                        p.requirements()
                                            .map(|er| er.req.license.id())
                                            .collect::<Vec<_>>()[0]
                                    }) {
                                        licenses.push(License {
                                            free: Some(
                                                license.is_osi_approved()
                                                    || license.is_fsf_free_libre(),
                                            ),
                                            fullname: license.full_name.to_string(),
                                            spdxid: Some(license.name.to_string()),
                                            url: Some(format!(
                                                "https://spdx.org/licenses/{}.html",
                                                license.name
                                            )),
                                        })
                                    }
                                }
                                LicenseEnum::VecStr(lst) => {
                                    for s in lst {
                                        addlicense(&LicenseEnum::SingleStr(s.clone()), licenses);
                                    }
                                }
                                LicenseEnum::Mixed(v) => {
                                    for l in v {
                                        addlicense(l, licenses);
                                    }
                                }
                            }
                        }

                        if let Ok(pkglicense) = serde_json::from_str::<LicenseEnum>(&licensejson) {
                            addlicense(&pkglicense, &mut licenses);
                        }

                        let platformslst = serde_json::from_str::<Platform>(&platformsjson);
                        if let Ok(p) = platformslst {
                            match p {
                                Platform::Single(p) => {
                                    if !platforms.contains(&p) && p != system {
                                        platforms.push(p);
                                    }
                                }
                                Platform::List(v) => {
                                    for p in v {
                                        if !platforms.contains(&p.to_string()) && p != system {
                                            platforms.push(p.to_string());
                                        }
                                    }
                                }
                                Platform::ListList(vv) => {
                                    for v in vv {
                                        for p in v {
                                            if !platforms.contains(&p.to_string()) && p != system {
                                                platforms.push(p.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        platforms.sort();
                        platforms.insert(0, system);

                        if let Ok(m) = serde_json::from_str::<Vec<PkgMaintainer>>(&maintainersjson)
                        {
                            for m in m {
                                maintainers.push(m);
                            }
                        }

                        let releases = app_data
                            .and_then(|ad| ad.releases.clone())
                            .unwrap_or_else(|| Vec::new());

                        let out = PkgInitModel {
                            name,
                            version: if version.is_empty() {
                                None
                            } else {
                                Some(version.to_string())
                            },
                            pname,
                            summary,
                            description,
                            icon,
                            pkg,
                            screenshots,
                            installeduserpkgs: self.installeduserpkgs.keys().cloned().collect(),
                            installedsystempkgs: self.installedsystempkgs.clone(),
                            launchable,
                            url,
                            releases,
                            position,
                            broken,
                            insecure,
                            unsupported,
                            unfree,
                        };
                        if self.viewstack.visible_child_name()
                            != Some(gtk::glib::GString::from("search"))
                        {
                            self.searching = false;
                        }
                        self.busy = false;
                        self.pkgpage.emit(PkgMsg::Open(Box::new(out)));

                        let page = self.pkgpage.widget();
                        self.navigation.push(page);
                    }
                } else {
                    error!("No pkgdb!");
                }
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
                if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", self.pkgdb)).await {
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
                            pkg: Some(installedpkg.clone()),
                            summary: summary.clone(),
                            icon: icon.clone(),
                            pkgtype: InstallType::User,
                            busy: self
                                .installedpagebusy
                                .contains(&(installedpkg.clone(), InstallType::User)),
                            version: version.to_string(),
                        });
                        if let Some(latest) = &self.nixpkgsdb
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
                                    pkg: Some(installedpkg.clone()),
                                    summary: summary.clone(),
                                    icon: icon.clone(),
                                    pkgtype: InstallType::System,
                                    busy: self
                                        .installedpagebusy
                                        .contains(&(installedpkg.clone(), InstallType::System)),
                                    version: version.to_string(),
                                });
                                if let Some(current) = &self.systemdb
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

                    self.installedpage.emit(InstalledPageMsg::Update(
                        installeduseritems,
                        installedsystemitems,
                    ));
                    self.updatepage
                        .emit(UpdatePageMsg::Update(updateuseritems, updatesystemitems));
                } else {
                    error!("Could not connect to pkgdb");
                }
            }
            AppMsg::UpdateCategoryPkgs => {
                self.categorypage.emit(CategoryPageMsg::UpdateInstalled(
                    self.installeduserpkgs.keys().cloned().collect::<Vec<_>>(),
                    self.installedsystempkgs.iter().cloned().collect::<Vec<_>>(),
                ));
            }
            AppMsg::SetSearch(show) => {
                self.set_searching(show);
                if !show
                    && let Some(s) = self.viewstack.visible_child_name()
                    && s == "search"
                {
                    self.viewstack.set_visible_child_name("explore");
                }
            }
            AppMsg::SetVsBar(vsbar) => {
                self.set_showvsbar(vsbar);
            }
            AppMsg::Search(search) => {
                info!("AppMsg::Search");
                debug!("Searching for: {}", search);
                self.viewstack.set_visible_child_name("search");
                self.set_searchquery(search.to_string());
                let installeduserpkgs = self.installeduserpkgs.clone();
                let installedsystempkgs = self.installedsystempkgs.clone();
                let pkgdb = self.pkgdb.clone();
                let appdata = self.appdata.clone();
                sender.command(move |out, shutdown| {
                    let search = search.clone();
                    let installeduserpkgs = installeduserpkgs.clone();
                    let installedsystempkgs = installedsystempkgs;
                    shutdown.register(async move {
                        let searchsplit: Vec<String> = search.split(' ').filter(|x| x.len() > 1).map(|x| x.to_string()).collect();
                        warn!("Searchsplit: {:?}", searchsplit);
                        if let Ok(pkgpool) = &SqlitePool::connect(&format!("sqlite://{}", pkgdb)).await {
                            let mut queryb: QueryBuilder<Sqlite> = QueryBuilder::new(
                                "SELECT pkgs.attribute, pkgs.pname, description, version FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE (",
                            );
                            for (i, q) in searchsplit.iter().enumerate() {
                                if i == searchsplit.len() - 1 {
                                    queryb
                                        .push(r#"pkgs.attribute LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#" OR description LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(")");
                                } else {
                                    queryb
                                        .push(r#"pkgs.attribute LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#" OR description LIKE "#)
                                        .push_bind(format!("%{}%", q))
                                        .push(r#") AND ("#);
                                }
                            }
                            queryb.push("ORDER BY LENGTH(pkgs.attribute) ASC");
                            let q: Vec<(String, String, String, String)> =
                                queryb.build_query_as().fetch_all(pkgpool).await.unwrap();
                            let mut outpkgs = Vec::new();
                            for (i, (attr, pname, desc, _version)) in q.into_iter().enumerate() {
                                if let Some(data) = appdata.get(&attr) {
                                    outpkgs.push(SearchItem {
                                        pkg: attr.to_string(),
                                        pname: pname.to_string(),
                                        name: if let Some(name) = &data.name { name.get("C").unwrap_or(&attr).to_string() } else { attr.to_string() },
                                        summary: if desc.is_empty() { None } else { Some(desc) },
                                        icon: data
                                            .icon
                                            .as_ref()
                                            .and_then(|x| x.cached.as_ref())
                                            .map(|x| x[0].name.clone()),
                                        installeduser: installeduserpkgs.contains_key(&attr),
                                        installedsystem: installedsystempkgs.contains(&attr),
                                    })
                                } else {
                                    outpkgs.push(SearchItem {
                                        pkg: attr.to_string(),
                                        pname: pname.to_string(),
                                        name: pname.to_string(),
                                        summary: if desc.is_empty() { None } else { Some(desc) },
                                        icon: None,
                                        installeduser: installeduserpkgs.contains_key(&attr),
                                        installedsystem: installedsystempkgs.contains(&attr),
                                    });
                                }
                                if i >= 200 {
                                    break;
                                }
                            }
                            outpkgs.sort_by(|a, b| {
                                let mut aleft = a.name.to_lowercase() + &a.pkg.to_lowercase();
                                let mut bleft = b.name.to_lowercase() + &b.pkg.to_lowercase();
                                for q in searchsplit.iter() {
                                    let q = &q.to_lowercase();
                                    if aleft.contains(q) {
                                        aleft = aleft.replace(q, "");
                                    } else {
                                        aleft.push_str(q);
                                    }
                                    if bleft.contains(q) {
                                        bleft = bleft.replace(q, "");
                                    } else {
                                        bleft.push_str(q);
                                    }
                                }
                                let mut apoints = aleft.len() + 5;
                                let mut bpoints = bleft.len() + 5;
                                // for q in searchsplit.iter() {
                                //     if a.name.contains(q) {
                                //         apoints -= 1;
                                //     }
                                //     if b.name.contains(q) {
                                //         bpoints -= 1;
                                //     }
                                // }
                                if appdata.contains_key(&a.pkg) {
                                    apoints -= 5;
                                }
                                if appdata.contains_key(&b.pkg) {
                                    bpoints -= 5;
                                }
                                apoints.cmp(&bpoints)
                            });
                            let _ = out.send(AppAsyncMsg::Search(search.to_string(), outpkgs));
                        }
                    }).drop_on_shutdown()
                })
            }
            AppMsg::AddInstalledToWorkQueue(work) => {
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy.push((p, work.pkgtype.clone()));
                self.pkgpage.emit(PkgMsg::AddToQueue(work));
            }
            AppMsg::RemoveInstalledBusy(work) => {
                let p = match work.pkgtype {
                    InstallType::User => work.pname.to_string(),
                    InstallType::System => work.pkg.to_string(),
                };
                self.installedpagebusy
                    .retain(|(x, y)| x != &p && y != &work.pkgtype);
                self.installedpage.emit(InstalledPageMsg::UnsetBusy(work));
            }
            AppMsg::OpenCategoryPage(category) => {
                info!("AppMsg::OpenCategoryPage({:?})", category);

                // open category page
                let page = self.categorypage.widget();
                self.navigation.push(page);

                self.categorypage
                    .emit(CategoryPageMsg::Loading(category.clone()));
                sender.input(AppMsg::LoadCategory(category));
            }
            AppMsg::LoadCategory(category) => {
                info!("AppMsg::LoadCategory({:?})", category);
                let pkgdb = self.pkgdb.clone();
                let categoryrec = self.categoryrec.get(&category).unwrap_or(&vec![]).to_vec();
                let categoryall = self.categoryall.get(&category).unwrap_or(&vec![]).to_vec();
                let appdata = self.appdata.clone();
                let installeduser = self.installeduserpkgs.clone();
                let installedsystem = self.installedsystempkgs.clone();

                sender.oneshot_command(async move {
                    let mut catrec = vec![];
                    let mut catall = vec![];
                    if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", pkgdb)).await {
                        for pkg in categoryrec {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catrec.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: if let Some(name) = &data.name {
                                        name.get("C").unwrap_or(&pname.0).to_string()
                                    } else {
                                        pname.0.to_string()
                                    },
                                    pname: pname.0,
                                    icon: data
                                        .icon
                                        .as_ref()
                                        .and_then(|x| x.cached.as_ref())
                                        .map(|x| x[0].name.clone()),
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string()),
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            } else {
                                let (pname, description): (String, String) =
                                sqlx::query_as("SELECT pname, description FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catrec.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: pname.to_string(),
                                    pname: pname.to_string(),
                                    icon: None,
                                    summary: if description.is_empty() { None } else { Some(description) },
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                        for pkg in categoryall {
                            if let Some(data) = appdata.get(&pkg) {
                                let pname: (String,) =
                                sqlx::query_as("SELECT pname FROM pkgs WHERE attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catall.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: if let Some(name) = &data.name {
                                        name.get("C").unwrap_or(&pname.0).to_string()
                                    } else {
                                        pname.0.to_string()
                                    },
                                    pname: pname.0,
                                    icon: data
                                        .icon
                                        .as_ref()
                                        .and_then(|x| x.cached.as_ref())
                                        .map(|x| x[0].name.clone()),
                                    summary: data
                                        .summary
                                        .as_ref()
                                        .and_then(|x| x.get("C"))
                                        .map(|x| x.to_string()),
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            } else {
                                let (pname, description): (String, String) =
                                sqlx::query_as("SELECT pname, description FROM pkgs JOIN meta ON (pkgs.attribute = meta.attribute) WHERE pkgs.attribute = $1")
                                    .bind(&pkg)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap();
                                catall.push(CategoryTile {
                                    pkg: pkg.to_string(),
                                    name: pname.to_string(),
                                    pname: pname.to_string(),
                                    icon: None,
                                    summary: if description.is_empty() { None } else { Some(description) },
                                    installeduser: installeduser.contains_key(&pkg),
                                    installedsystem: installedsystem.contains(&pkg),
                                })
                            }
                        }
                    } else {
                        error!("Failed to connect to pkgdb")
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
                let poolref = self.pkgdb.clone();
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
            AppAsyncMsg::Search(search, pkgitems) => {
                if search == self.searchquery {
                    self.searchpage.emit(SearchPageMsg::Search(pkgitems))
                }
            }
            AppAsyncMsg::UpdateRecPkgs(pkgtiles, pkg_category) => {
                self.explore_page
                    .emit(ExplorePageMsg::UpdateRecommendedPackages(
                        pkgtiles,
                        pkg_category,
                    ));
            }
            AppAsyncMsg::UpdateInstalledPkgs(installedsystempkgs, installeduserpkgs) => {
                // TODO: maybe create macro to update installed pkgs
                info!("AppAsyncMsg::UpdateInstalledPkgs");
                if installedsystempkgs != self.installedsystempkgs
                    || installeduserpkgs != self.installeduserpkgs
                {
                    warn!("Changes needed!");
                    self.installedsystempkgs = installedsystempkgs.clone();
                    self.installeduserpkgs = installeduserpkgs.clone();

                    self.explore_page
                        .emit(ExplorePageMsg::UpdateInstalledPackages(
                            installedsystempkgs,
                            installeduserpkgs,
                        ));

                    if self.searching {
                        self.searchpage.emit(SearchPageMsg::UpdateInstalled(
                            self.installeduserpkgs.keys().cloned().collect(),
                            self.installedsystempkgs.clone(),
                        ));
                    }
                }
                // Always refresh the update page
                sender.input(AppMsg::UpdateInstalledPage);
                info!("DONE AppAsyncMsg::UpdateInstalledPkgs");
            }
            AppAsyncMsg::LoadCategory(category, catrec, catall) => {
                self.categorypage
                    .emit(CategoryPageMsg::Open(category, catrec, catall));
            }
            AppAsyncMsg::SetNetwork(online) => {
                self.online = online;
                self.updatepage.emit(UpdatePageMsg::UpdateOnline(online));
                self.pkgpage.emit(PkgMsg::UpdateOnline(online));
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

relm4::new_action_group!(MenuActionGroup, "menu");
relm4::new_stateless_action!(AboutAction, MenuActionGroup, "about");
relm4::new_stateless_action!(PreferencesAction, MenuActionGroup, "preferences");
