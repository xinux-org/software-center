use adw::gio;
use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use html2pango;
use image::{ImageFormat, imageops::FilterType};
use log::*;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::actions::RelmAction;
use relm4::actions::RelmActionGroup;
use relm4::component::Connector;
use relm4::gtk::pango;
use relm4::{factory::FactoryVecDeque, *};
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::collections::HashSet;
use std::convert::identity;
use std::io::Cursor;
use std::process::Command;
use std::{
    env,
    error::Error,
    fmt::Write,
    fs::{self, File},
    io::BufReader,
    path::Path,
    time::Duration,
};

use super::components::{
    link_item::{LinkItem, LinkItemInit, LinkType},
    release_item::{ReleaseItem, ReleaseItemInit},
    releases_dialog::ReleasesDialog,
};
use crate::ui::package::components::releases_dialog::ReleasesInit;
use crate::{
    ui::{
        installed::install_worker::{
            InstallAsyncHandler, InstallAsyncHandlerInit, InstallAsyncHandlerMsg,
        },
        package::components::screenshot::ScreenshotItem,
        window::{AppMsg, SystemPkgs},
    },
    utils::{
        packages::{AppRelease, AppUrl, ReleaseType},
        {online::checkonline, packages::PkgMaintainer, state},
    },
};

#[tracker::track]
#[derive(Debug)]
pub struct PkgModel {
    config: NixDataConfig,
    name: String,
    pkg: String,
    pname: String,
    summary: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    version: Option<String>,

    homepage: Option<String>,
    licenses: Vec<License>,
    platforms: Vec<String>,
    maintainers: Vec<PkgMaintainer>,
    launchable: Option<Launch>,

    syspkgtype: SystemPkgs,

    #[tracker::no_eq]
    screenshots: FactoryVecDeque<ScreenshotItem>,
    #[tracker::no_eq]
    links: FactoryVecDeque<LinkItem>,
    #[tracker::no_eq]
    latest_release: FactoryVecDeque<ReleaseItem>,
    #[tracker::no_eq]
    installworker: WorkerController<InstallAsyncHandler>,

    #[tracker::no_eq]
    releases_dialog: Option<Connector<ReleasesDialog>>,

    carpage: CarouselPage,
    installtype: InstallType,
    installed_pkgs: HashSet<String>,
    installeduserpkgs: HashSet<String>,
    installedsystempkgs: HashSet<String>,

    workqueue: HashSet<WorkPkg>,
    visible: bool,
    online: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct WorkPkg {
    pub pkg: String,
    pub pname: String,
    pub pkgtype: InstallType,
    pub action: PkgAction,
    pub block: bool,
    pub notify: Option<NotifyPage>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum NotifyPage {
    Installed,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PkgAction {
    Install,
    Remove,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Launch {
    GtkApp(String),
    TerminalApp(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CarouselPage {
    First,
    Middle,
    Last,
    Single,
}

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Clone)]
pub enum InstallType {
    User,
    System,
}

#[derive(Debug, PartialEq, Eq)]
pub struct License {
    pub free: Option<bool>,
    pub fullname: String,
    pub spdxid: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug)]
pub struct PkgInitModel {
    pub name: String,
    pub pkg: String,
    pub installeduserpkgs: HashSet<String>,
    pub installedsystempkgs: HashSet<String>,
    pub pname: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub icon: Option<String>,
    pub screenshots: Vec<String>,
    pub homepage: Option<String>,
    pub licenses: Vec<License>,
    pub platforms: Vec<String>,
    pub maintainers: Vec<PkgMaintainer>,
    pub launchable: Option<String>,
    pub url: Option<AppUrl>,
    pub releases: Vec<AppRelease>,
}

#[derive(Debug)]
pub enum PkgMsg {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs),
    Open(Box<PkgInitModel>),
    LoadScreenshot(String, usize, String),
    SetError(String, usize),
    SetCarouselPage(CarouselPage),
    OpenHomepage,
    Install,
    Remove,
    Cancel,
    CancelFinished,
    FinishedProcess(WorkPkg),
    FailedProcess(WorkPkg),
    Launch,
    NixRun,
    NixShell,
    SetInstallType(InstallType),
    AddToQueue(WorkPkg),
    UpdateOnline(bool),
    ShowReleases,
    Noop,
}

#[derive(Debug)]
pub enum PkgAsyncMsg {
    LoadScreenshot(String, usize, String),
    SetError(String, usize),
}

#[derive(Debug)]
pub struct PkgPageInit {
    pub syspkgs: SystemPkgs,
    pub config: NixDataConfig,
    pub online: bool,
}

#[relm4::component(pub)]
impl Component for PkgModel {
    type Init = PkgPageInit;
    type Input = PkgMsg;
    type Output = AppMsg;
    type CommandOutput = PkgAsyncMsg;

    view! {
        #[root]
        #[name(pkg_window)]
        adw::NavigationPage {

            #[watch]
            set_title: &model.name,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                adw::HeaderBar {
                    pack_end = &gtk::MenuButton {
                        #[watch]
                        set_visible: model.syspkgtype != SystemPkgs::None,

                        #[watch]
                        set_label: &match model.installtype {
                            InstallType::User =>  gettext("User (nix profile)"),
                            InstallType::System => gettext("System (configuration.nix)"),
                        },

                        #[wrap(Some)]
                        set_popover = &gtk::PopoverMenu::from_model(Some(&installtype)) {}
                    }
                },
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vscrollbar_policy: gtk::PolicyType::Automatic,
                    #[track(model.changed(PkgModel::visible()) && !self.visible)]
                    set_vadjustment: gtk::Adjustment::NONE,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        adw::Clamp {
                            set_maximum_size: 1000,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            // Details box
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 10,
                                set_margin_all: 15,
                                append = if model.icon.is_some() {
                                    gtk::Image {
                                        add_css_class: "icon-dropshadow",
                                        set_halign: gtk::Align::Start,
                                        #[watch]
                                        set_from_file: model.icon.clone(),
                                        set_pixel_size: 128,
                                    }
                                } else {
                                    gtk::Image {
                                        add_css_class: "icon-dropshadow",
                                        set_halign: gtk::Align::Start,
                                        set_icon_name: Some("package-x-generic"),
                                        set_pixel_size: 128,
                                    }
                                },
                                gtk::FlowBox {
                                    set_halign: gtk::Align::Fill,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_min_children_per_line: 1,
                                    set_max_children_per_line: 2,
                                    set_selection_mode: gtk::SelectionMode::None,
                                    // Details
                                    append = &gtk::FlowBoxChild {
                                        set_can_target: false,
                                        gtk::Box {
                                            set_halign: gtk::Align::Fill,
                                            set_valign: gtk::Align::Center,
                                            set_hexpand: true,
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 6,
                                            gtk::Label {
                                                add_css_class: "title-1",
                                                set_halign: gtk::Align::Start,
                                                set_wrap: true,
                                                set_wrap_mode: pango::WrapMode::WordChar,
                                                set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                                #[watch]
                                                set_label: &model.name,
                                            },
                                            gtk::Label {
                                                add_css_class: "dim-label",
                                                add_css_class: "heading",
                                                set_halign: gtk::Align::Start,
                                                set_wrap: true,
                                                set_wrap_mode: pango::WrapMode::WordChar,
                                                set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                                #[watch]
                                                set_label: &model.pkg,
                                            },
                                            gtk::Label {
                                                add_css_class: "dim-label",
                                                set_halign: gtk::Align::Start,
                                                set_wrap: true,
                                                set_wrap_mode: pango::WrapMode::WordChar,
                                                set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                                #[watch]
                                                set_label: &model.version.clone().unwrap_or_else(||  gettext("Unknown").to_string()),
                                            },
                                        },
                                    },

                                    // Install options
                                    append = &gtk::FlowBoxChild {
                                        set_halign: gtk::Align::End,
                                        gtk::Box {
                                            set_halign: gtk::Align::End,
                                            set_spacing: 5,
                                            #[name(install_stack)]
                                            if model.workqueue.iter().any(|x| x.pkg == model.pkg && x.pkgtype == model.installtype) {
                                                gtk::Box {
                                                    set_halign: gtk::Align::End,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 10,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "pill",
                                                        set_width_request: 105,
                                                        set_sensitive: false,
                                                        gtk::Box {
                                                            set_halign: gtk::Align::Center,
                                                            set_spacing: 10,
                                                            gtk::Spinner {
                                                                set_spinning: true,
                                                                set_size_request: (24, 24),
                                                                set_can_focus: false,
                                                            },
                                                            gtk::Label {
                                                                 set_label: &gettext("Installing..."),
                                                            },
                                                        }
                                                    },
                                                    gtk::Button {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Fill,
                                                        add_css_class: "destructive-action",
                                                        add_css_class: "circular",
                                                        set_icon_name: "process-stop-symbolic",
                                                        set_width_request: 44,
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PkgMsg::Cancel)
                                                        },
                                                    }
                                                }
                                            } else if model.installed_pkgs.contains(&model.pkg) {
                                                gtk::Box {
                                                    set_halign: gtk::Align::End,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 10,
                                                    gtk::Button {
                                                        #[watch]
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "pill",
                                                        set_width_request: 105,
                                                        #[watch]
                                                        set_label: &if model.launchable.is_some() { gettext("Open") } else { gettext("Installed") },
                                                        #[watch]
                                                        set_sensitive: model.launchable.is_some(),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PkgMsg::Launch)
                                                        }
                                                    },
                                                    gtk::Button {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Fill,
                                                        add_css_class: "destructive-action",
                                                        add_css_class: "circular",
                                                        set_icon_name: "user-trash-symbolic",
                                                        set_width_request: 44,
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PkgMsg::Remove)
                                                        }
                                                    }
                                                }
                                            } else if !model.online {
                                                gtk::Box {
                                                    set_spacing: 10,
                                                    set_halign: gtk::Align::End,
                                                    set_valign: gtk::Align::Center,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "error",
                                                        add_css_class: "pill",
                                                        set_width_request: 105,
                                                        set_label: &gettext("Offline"),
                                                        set_can_target: false,
                                                        set_can_focus: false,
                                                    },
                                                    gtk::Button {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Fill,
                                                        add_css_class: "circular",
                                                        set_icon_name: "nsc-refresh-symbolic",
                                                        set_width_request: 44,
                                                        connect_clicked[sender] => move |_| {
                                                            let _ = sender.output(AppMsg::CheckNetwork);
                                                        }
                                                    }
                                                }
                                            } else {
                                                gtk::Box {
                                                    set_halign: gtk::Align::End,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 10,
                                                    gtk::Button {
                                                        #[watch]
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "suggested-action",
                                                        add_css_class: "pill",
                                                        set_width_request: 105,
                                                        set_label: &gettext("Install"),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PkgMsg::Install);
                                                        },
                                                    },
                                                    gtk::MenuButton {
                                                        set_halign: gtk::Align::End,
                                                        set_valign: gtk::Align::Fill,
                                                        add_css_class: "circular",
                                                        set_icon_name: "view-more-symbolic",
                                                        set_width_request: 44,
                                                        #[wrap(Some)]
                                                        set_popover = &gtk::PopoverMenu::from_model(Some(&runaction)) {},
                                                    },
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Start,
                            add_css_class: "view",
                            add_css_class: "frame",
                            add_css_class: "scrnbox",
                            #[watch]
                            set_visible: !model.screenshots.is_empty(),
                            gtk::Overlay {
                                set_valign: gtk::Align::Start,
                                #[local_ref]
                                scrnfactory -> adw::Carousel {
                                    set_valign: gtk::Align::Fill,
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_height_request: 400,
                                    set_allow_scroll_wheel: false,
                                    connect_page_changed[sender] => move |x, _| {
                                        let n = adw::Carousel::n_pages(x);
                                        let i = adw::Carousel::position(x) as u32;
                                        if i == 0 && n == 1 {
                                            sender.input(PkgMsg::SetCarouselPage(CarouselPage::Single));
                                        } else if i == 0 {
                                            sender.input(PkgMsg::SetCarouselPage(CarouselPage::First));
                                        } else if i == n - 1 {
                                            sender.input(PkgMsg::SetCarouselPage(CarouselPage::Last));
                                        } else {
                                            sender.input(PkgMsg::SetCarouselPage(CarouselPage::Middle));
                                        }
                                    },
                                },
                                add_overlay = &gtk::Revealer {
                                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                                    #[watch]
                                    set_reveal_child: model.carpage != CarouselPage::First && model.carpage != CarouselPage::Single,
                                    set_halign: gtk::Align::Start,
                                    set_valign: gtk::Align::Fill,
                                    gtk::Button {
                                        set_can_focus: false,
                                        set_margin_all: 15,
                                        set_height_request: 40,
                                        set_width_request: 40,
                                        add_css_class: "circular",
                                        add_css_class: "osd",
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Center,
                                        set_icon_name: "go-previous-symbolic",
                                        connect_clicked[sender, scrnfactory] => move |_| {
                                            let i = adw::Carousel::position(&scrnfactory) as u32;
                                            if i > 0 {
                                                let w = scrnfactory.nth_page(i-1);
                                                scrnfactory.scroll_to(&w, true);
                                            }
                                            if i == 1 {
                                                sender.input(PkgMsg::SetCarouselPage(CarouselPage::First));
                                            } else if i > 0 {
                                                sender.input(PkgMsg::SetCarouselPage(CarouselPage::Middle));
                                            }
                                        }
                                    }
                                },
                                add_overlay = &gtk::Revealer {
                                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                                    #[watch]
                                    set_reveal_child: model.carpage != CarouselPage::Last && model.carpage != CarouselPage::Single,
                                    set_halign: gtk::Align::End,
                                    set_valign: gtk::Align::Fill,
                                    gtk::Button {
                                        set_can_focus: false,
                                        set_margin_all: 15,
                                        set_height_request: 40,
                                        set_width_request: 40,
                                        add_css_class: "circular",
                                        add_css_class: "osd",
                                        set_halign: gtk::Align::End,
                                        set_valign: gtk::Align::Center,
                                        set_icon_name: "go-next-symbolic",
                                        connect_clicked[sender, scrnfactory] => move |_| {
                                            let i = adw::Carousel::position(&scrnfactory) as u32;
                                            if i < scrnfactory.n_pages() -1 {
                                                let w = scrnfactory.nth_page(i+1);
                                                scrnfactory.scroll_to(&w, true);
                                            }
                                            let n = scrnfactory.n_pages();
                                            if i == n - 2 {
                                                sender.input(PkgMsg::SetCarouselPage(CarouselPage::Last));
                                            } else if i <= n - 2 {
                                                sender.input(PkgMsg::SetCarouselPage(CarouselPage::Middle));
                                            } else {
                                                sender.input(PkgMsg::SetCarouselPage(CarouselPage::Last));
                                            }
                                        }
                                    }
                                }
                            },
                            adw::CarouselIndicatorDots {
                                set_halign: gtk::Align::Fill,
                                set_valign: gtk::Align::End,
                                set_carousel: Some(scrnfactory)
                            }
                        },
                        adw::Clamp {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            set_vexpand_set: true,
                            set_maximum_size: 1000,
                            #[watch]
                            set_visible: !(model.summary.is_none() && model.description.is_none()),
                            gtk::Box {
                                set_vexpand: true,
                                set_valign: gtk::Align::Start,
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 15,
                                set_spacing: 10,
                                gtk::Label {
                                    add_css_class: "title-2",
                                    set_valign: gtk::Align::Start,
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_label: if let Some(s) = model.summary.as_ref() { s } else { "" },
                                    #[watch]
                                    set_visible: model.summary.is_some(),
                                    set_wrap: true,
                                    set_xalign: 0.0,
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Start,
                                    set_halign: gtk::Align::Start,
                                    #[watch]
                                    set_markup: {
                                        if let Some(d) = model.description.as_ref() {
                                            d
                                        } else { "" }
                                    },
                                    #[watch]
                                    set_visible: model.description.is_some(),
                                    set_wrap: true,
                                    set_xalign: 0.0,
                                },
                            },
                        },
                        adw::Clamp {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            set_vexpand: true,
                            set_maximum_size: 1000,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,
                                #[local_ref]
                                latest_release_factory -> adw::PreferencesGroup {},
                                adw::PreferencesGroup {
                                    adw::ButtonRow {
                                        set_title: &gettext("Version History"),
                                        set_end_icon_name: Some("right-symbolic"),
                                        connect_activated[sender] => move |_| {
                                            sender.input(PkgMsg::ShowReleases);
                                        }
                                    }
                                }
                            },
                        },
                        adw::Clamp {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            set_vexpand: true,
                            set_maximum_size: 1000,
                            gtk::Box {
                                set_vexpand: true,
                                set_valign: gtk::Align::Start,
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 15,
                                set_spacing: 10,
                                gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    add_css_class: "title-2",
                                    set_label: &gettext("Links"),
                                },
                                gtk::Box {
                                    set_spacing: 12,
                                    #[local_ref]
                                    link_factory -> adw::PreferencesGroup {
                                        set_hexpand: true,
                                    },
                                },
                            },
                        },
                        adw::Clamp {
                            set_vexpand: true,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Start,
                            set_maximum_size: 1000,
                            #[name(btnbox)]
                            gtk::FlowBox {
                                add_css_class: "linked",
                                set_halign: gtk::Align::Fill,
                                set_hexpand: true,
                                set_margin_bottom: 10,
                                set_homogeneous: true,
                                set_row_spacing: 5,
                                set_column_spacing: 4,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_max_children_per_line: 4,
                                append = &gtk::FlowBoxChild {
                                    // set_can_target: false,
                                    set_hexpand: true,
                                    gtk::Box {
                                        set_spacing: 10,
                                        set_hexpand: true,
                                        set_homogeneous: true,
                                        gtk::Button {
                                            set_hexpand: true,
                                            add_css_class: "card",
                                            set_height_request: 100,
                                            set_width_request: 100,
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Center,
                                                set_spacing: 10,
                                                set_margin_all: 15,
                                                gtk::Image {
                                                    #[watch]
                                                    set_css_classes: &[ if model.licenses.iter().any(|x| x.free == Some(false)) { "error" } else if model.licenses.iter().all(|x| x.free == Some(true)) { "success" } else { "warning" } ],
                                                    set_halign: gtk::Align::Center,
                                                    #[watch]
                                                    set_icon_name: if model.licenses.iter().any(|x| x.free == Some(false)) { Some("dialog-warning-symbolic") } else if model.licenses.iter().all(|x| x.free == Some(true)) { Some("emblem-default-symbolic") } else { Some("dialog-question-symbolic") },
                                                    set_pixel_size: 24,
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_halign: gtk::Align::Fill,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 5,
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "heading",
                                                        #[watch]
                                                        set_label: &if model.licenses.len() > 1 { gettext("Licenses") } else { gettext("License") }
                                                    },
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Fill,
                                                        set_hexpand: true,
                                                        add_css_class: "caption",
                                                        add_css_class: "dim-label",
                                                        set_ellipsize: pango::EllipsizeMode::End,
                                                        set_lines: 2,
                                                        set_wrap: true,
                                                        set_max_width_chars: 0,
                                                        set_justify: gtk::Justification::Center,
                                                        #[watch]
                                                        set_label: &{
                                                            let mut s = String::new();
                                                            for license in model.licenses.iter() {
                                                                if model.licenses.iter().len() == 1 {
                                                                    if let Some(id) = &license.spdxid {
                                                                        s.push_str(id)
                                                                    } else {
                                                                        s.push_str(&license.fullname)
                                                                    }
                                                                } else if model.licenses.iter().len() == 2 && model.licenses.first() == Some(license) {
                                                                    if let Some(id) = &license.spdxid {
                                                                        let _ = write!(s, "{} ", id);
                                                                    } else {
                                                                        let _ = write!(s, "{} ", license.fullname);
                                                                    }
                                                                } else if Some(license) == model.licenses.iter().last() {
                                                                    if let Some(id) = &license.spdxid {
                                                                        let _ = write!(s, "and {}", id);
                                                                    } else {
                                                                        let _ = write!(s, "and {}", license.fullname);
                                                                    }
                                                                } else if let Some(id) = &license.spdxid {
                                                                    let _ = write!(s, "{}, ", id);
                                                                } else {
                                                                    let _ = write!(s, "{}, ", license.fullname);
                                                                }
                                                            }
                                                            if model.licenses.is_empty() {
                                                                s.push_str(&gettext("Unknown"));
                                                            }
                                                            s.to_string()
                                                        },
                                                        #[watch]
                                                        set_visible: !model.licenses.is_empty()
                                                    }
                                                }
                                            }
                                        },
                                    }
                                },
                                append = &gtk::FlowBoxChild {
                                    set_hexpand: true,
                                    gtk::Box {
                                        set_spacing: 10,
                                        set_hexpand: true,
                                        set_homogeneous: true,
                                        gtk::Button {
                                            set_hexpand: true,
                                            add_css_class: "card",
                                            set_height_request: 100,
                                            set_width_request: 100,
                                            connect_clicked[sender] => move |_| {
                                                sender.input(PkgMsg::OpenHomepage)
                                            },
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Center,
                                                set_spacing: 10,
                                                set_margin_all: 15,
                                                gtk::Image {
                                                    add_css_class: "accent",
                                                    set_halign: gtk::Align::Center,
                                                    set_icon_name: Some("user-home-symbolic"),
                                                    set_pixel_size: 24,
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_halign: gtk::Align::Fill,
                                                    set_valign: gtk::Align::Center,
                                                    set_hexpand: true,
                                                    set_spacing: 5,
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Center,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "heading",
                                                        set_label: &gettext("Homepage")
                                                    },
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Fill,
                                                        set_valign: gtk::Align::Center,
                                                        add_css_class: "caption",
                                                        add_css_class: "dim-label",
                                                        set_ellipsize: pango::EllipsizeMode::End,
                                                        set_lines: 2,
                                                        set_wrap: true,
                                                        set_max_width_chars: 0,
                                                        set_justify: gtk::Justification::Center,
                                                        #[watch]
                                                        set_label: if let Some(u) = &model.homepage {
                                                            u
                                                        } else {
                                                            ""
                                                        },
                                                        #[watch]
                                                        set_visible: model.homepage.is_some(),
                                                    }
                                                }

                                            }
                                        },
                                    }
                                },
                                append = &gtk::FlowBoxChild {
                                    set_hexpand: true,
                                    gtk::Box {
                                        set_spacing: 10,
                                        set_hexpand: true,
                                        set_homogeneous: true,
                                        gtk::Button {
                                            set_hexpand: true,
                                            add_css_class: "card",
                                            set_height_request: 100,
                                            set_width_request: 100,
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_valign: gtk::Align::Center,
                                                set_spacing: 10,
                                                set_margin_all: 15,
                                                gtk::Image {
                                                    add_css_class: "success",
                                                    set_icon_name: Some("video-display-symbolic"),
                                                    set_pixel_size: 24,
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 5,
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "heading",
                                                        set_label: &gettext("Platforms")
                                                    },
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Fill,
                                                        set_hexpand: true,
                                                        add_css_class: "caption",
                                                        add_css_class: "dim-label",
                                                        set_ellipsize: pango::EllipsizeMode::End,
                                                        set_lines: 2,
                                                        set_wrap: true,
                                                        set_max_width_chars: 0,
                                                        set_justify: gtk::Justification::Center,
                                                        #[watch]
                                                        set_label: &{
                                                            let mut s = String::new();
                                                            for p in model.platforms.iter() {
                                                                if model.platforms.iter().len() == 1 {
                                                                    s.push_str(p);
                                                                } else if model.platforms.iter().len() == 2 && model.platforms.first() == Some(p) {
                                                                    let _ = write!(s, "{} ", p);
                                                                } else if Some(p) == model.platforms.iter().last() {
                                                                    let _ = write!(s, "and {}", p);
                                                                } else {
                                                                    let _ = write!(s, "{}, ", p);
                                                                }
                                                            }
                                                            if model.platforms.is_empty() {
                                                                s.push_str(&gettext("Unknown"));
                                                            }
                                                            s.to_string()
                                                        },
                                                        #[watch]
                                                        set_visible: !model.platforms.is_empty()
                                                    }
                                                }
                                            }
                                        },
                                    }
                                },
                                append = &gtk::FlowBoxChild {
                                    set_hexpand: true,
                                    gtk::Box {
                                        set_spacing: 10,
                                        set_hexpand: true,
                                        set_homogeneous: true,

                                        gtk::Button {
                                            set_hexpand: true,
                                            add_css_class: "card",
                                            set_height_request: 100,
                                            set_width_request: 100,
                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_halign: gtk::Align::Fill,
                                                set_valign: gtk::Align::Center,
                                                set_spacing: 10,
                                                set_margin_all: 15,
                                                gtk::Image {
                                                    add_css_class: "circular",
                                                    #[watch]
                                                    set_css_classes: &[ if model.maintainers.is_empty() { "error" } else { "accent" } ],
                                                    set_halign: gtk::Align::Center,
                                                    set_icon_name: Some("system-users-symbolic"),
                                                    set_pixel_size: 24,
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_valign: gtk::Align::Center,
                                                    set_spacing: 5,
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "heading",
                                                        #[watch]
                                                        set_label: &if model.maintainers.len() > 1 { gettext("Maintainers") } else { gettext("Maintainer") }
                                                    },
                                                    gtk::Label {
                                                        set_halign: gtk::Align::Fill,
                                                        set_hexpand: true,
                                                        add_css_class: "caption",
                                                        add_css_class: "dim-label",
                                                        set_ellipsize: pango::EllipsizeMode::End,
                                                        set_lines: 2,
                                                        set_wrap: true,
                                                        set_max_width_chars: 0,
                                                        set_justify: gtk::Justification::Center,
                                                        #[watch]
                                                        set_label: &{
                                                            let mut s = String::new();
                                                            let maintainerlist = model.maintainers.iter().filter(|m| m.name.is_some() || m.github.is_some()).collect::<Vec<_>>();
                                                            for p in &maintainerlist {
                                                                if maintainerlist.len() == 1 {
                                                                    if let Some(n) = &p.name {
                                                                        s.push_str(n);
                                                                    } else if let Some(g) = &p.github {
                                                                        s.push_str(g);
                                                                    }
                                                                } else if maintainerlist.len() == 2 && model.maintainers.first() == Some(p) {
                                                                    if let Some(n) = &p.name {
                                                                        let _ = write!(s, "{} ", n.as_str());
                                                                    } else if let Some(g) = &p.github {
                                                                        s.push_str(g);
                                                                    }
                                                                } else if Some(p) == maintainerlist.last() {
                                                                    if let Some(n) = &p.name {
                                                                        let _ = write!(s, "and {}", n.as_str());
                                                                    } else if let Some(g) = &p.github {
                                                                        let _ = write!(s, "and {}", g.as_str());
                                                                    }
                                                                } else if let Some(n) = &p.name {
                                                                    let _ = write!(s, "{}, ", n.as_str());
                                                                } else if let Some(g) = &p.github {
                                                                    let _ = write!(s, "{}, ", g.as_str());
                                                                }
                                                            }
                                                            if model.maintainers.is_empty() {
                                                                s.push_str(&gettext("Unknown"));
                                                            }
                                                            s.to_string()
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                    }
                                },
                            }
                        },
                        gtk::Separator {
                            set_vexpand: true,
                            add_css_class: "spacer"
                        }
                    }
                }
            }
        }
    }

    menu! {
        installtype: {
            &gettext("User (nix profile)") => NixProfileAction,
            &gettext("System (configuration.nix)") => NixSystemAction,
        },
        runaction: {
            &gettext("Run without installing") => LaunchAction,
            &gettext("Open interactive shell") => TermShellAction,
        }
    }

    fn init(
        initparams: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let installworker = InstallAsyncHandler::builder()
            .detach_worker(InstallAsyncHandlerInit {
                syspkgs: initparams.syspkgs.clone(),
            })
            .forward(sender.input_sender(), identity);
        let config = initparams.config;
        installworker.emit(InstallAsyncHandlerMsg::SetConfig(config.clone()));

        let model = PkgModel {
            config,
            name: String::default(),
            pkg: String::default(),
            pname: String::default(),
            summary: None,
            description: None,
            version: None,
            icon: None,
            homepage: None,
            licenses: vec![],
            screenshots: FactoryVecDeque::builder()
                .launch(adw::Carousel::new())
                .forward(sender.input_sender(), |_| PkgMsg::Noop),
            links: FactoryVecDeque::builder()
                .launch(adw::PreferencesGroup::new())
                .forward(sender.input_sender(), |_| PkgMsg::Noop),
            latest_release: FactoryVecDeque::builder()
                .launch(adw::PreferencesGroup::new())
                .forward(sender.input_sender(), |_| PkgMsg::Noop),
            installworker,

            releases_dialog: None,

            platforms: vec![],
            carpage: CarouselPage::Single,
            installtype: InstallType::User,
            maintainers: vec![],
            installed_pkgs: HashSet::new(),
            installeduserpkgs: HashSet::new(),
            installedsystempkgs: HashSet::new(),
            syspkgtype: initparams.syspkgs,
            workqueue: HashSet::new(),
            launchable: None,
            visible: false,
            online: initparams.online,
            tracker: 0,
        };

        let link_factory = model.links.widget();

        let latest_release_factory = model.latest_release.widget();

        let scrnfactory = model.screenshots.widget();
        relm4::set_global_css(
            ".scrnbox {
            border-left-width: 0;
            border-right-width: 0;
            border-top-width: 1px;
            border-bottom-width: 1px;
        }",
        );
        let widgets = view_output!();
        widgets.install_stack.set_hhomogeneous(false);

        let mut group = RelmActionGroup::<ModeActionGroup>::new();

        let nixprofile: RelmAction<NixProfileAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PkgMsg::SetInstallType(InstallType::User));
            })
        };

        let nixsystem: RelmAction<NixSystemAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PkgMsg::SetInstallType(InstallType::System));
            })
        };

        group.add_action(nixprofile);
        group.add_action(nixsystem);

        let actions = group.into_action_group();
        widgets
            .pkg_window
            .insert_action_group("mode", Some(&actions));

        let mut rungroup = RelmActionGroup::<RunActionGroup>::new();
        let launchaction: RelmAction<LaunchAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PkgMsg::NixRun);
            })
        };

        let termaction: RelmAction<TermShellAction> = {
            let sender = sender;
            RelmAction::new_stateless(move |_| sender.input(PkgMsg::NixShell))
        };

        rungroup.add_action(launchaction);
        rungroup.add_action(termaction);

        let runactions = rungroup.into_action_group();
        widgets
            .pkg_window
            .insert_action_group("run", Some(&runactions));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        self.reset();
        match msg {
            PkgMsg::UpdateConfig(config) => {
                self.config = config.clone();
                self.installworker
                    .emit(InstallAsyncHandlerMsg::SetConfig(config));
            }
            PkgMsg::UpdatePkgTypes(syspkgs) => {
                self.syspkgtype = syspkgs.clone();
                self.installworker
                    .emit(InstallAsyncHandlerMsg::SetPkgTypes(syspkgs));
            }
            PkgMsg::Open(pkgmodel) => {
                // First clean up from previous package
                self.summary = None;
                self.description = None;
                self.icon = None;
                let mut scrn_guard = self.screenshots.guard();
                scrn_guard.clear();
                scrn_guard.drop();

                self.set_visible(true);
                self.set_pkg(pkgmodel.pkg);
                self.set_name(pkgmodel.name);
                self.set_icon(pkgmodel.icon);
                self.set_version(pkgmodel.version);
                self.set_platforms(pkgmodel.platforms);
                self.set_maintainers(pkgmodel.maintainers);
                self.set_licenses(pkgmodel.licenses);
                self.set_pname(pkgmodel.pname);
                self.set_installeduserpkgs(pkgmodel.installeduserpkgs);
                self.set_installedsystempkgs(pkgmodel.installedsystempkgs);

                let is_system_pkg = self.get_installedsystempkgs().contains(&self.pkg);
                let is_user_pkg = self.get_installeduserpkgs().contains(&self.pkg);
                match (is_system_pkg, is_user_pkg) {
                    (true, false) => self.set_installtype(InstallType::System),
                    (false, true) => self.set_installtype(InstallType::User),
                    _ => {
                        let install_type = state::get_state()
                            .and_then(|state| state.install_type)
                            .unwrap_or(InstallType::User);
                        self.set_installtype(install_type);
                    }
                }

                self.set_installed_pkgs(match self.installtype {
                    InstallType::System => self.installedsystempkgs.clone(),
                    InstallType::User => self.installeduserpkgs.clone(),
                });

                self.launchable = if let Some(l) = pkgmodel.launchable {
                    Some(Launch::GtkApp(l))
                } else if self.installeduserpkgs.contains(&self.pkg) {
                    if let Ok(o) = Command::new("command").arg("-v").arg(&self.pname).output() {
                        if o.status.success() {
                            Some(Launch::TerminalApp(self.pname.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.summary = if let Some(s) = pkgmodel.summary {
                    let mut sum = s.trim().to_string();
                    while sum.contains('\n') {
                        sum = sum.replace('\n', " ");
                    }
                    while sum.contains("  ") {
                        sum = sum.replace("  ", " ");
                    }
                    Some(sum)
                } else {
                    None
                };

                if let Some(d) = pkgmodel.description {
                    let mut input = d;
                    // Fix formatting
                    while input.contains('\n') {
                        input = input.replace('\n', " ");
                    }
                    while input.contains('\t') {
                        input = input.replace('\t', " ");
                    }
                    while input.contains("  ") {
                        input = input.replace("  ", " ");
                    }
                    let mut pango = html2pango::markup_html(&input)
                        .unwrap_or_else(|_| {
                            warn!("Pango failed to parse description");
                            input.to_string()
                        })
                        .trim()
                        .to_string();
                    while pango.contains("\n ") {
                        pango = pango.replace("\n ", "\n");
                    }
                    while pango.ends_with('\n') {
                        pango.pop();
                    }
                    self.description = Some(pango.strip_prefix('\n').unwrap_or(&pango).to_string());
                }

                self.homepage = pkgmodel.homepage;

                if pkgmodel.screenshots.len() <= 1 {
                    self.carpage = CarouselPage::Single;
                } else {
                    self.carpage = CarouselPage::First;
                }

                {
                    let mut scrn_guard = self.screenshots.guard();
                    scrn_guard.clear();
                    for _i in 0..pkgmodel.screenshots.len() {
                        scrn_guard.push_back(());
                    }
                }

                {
                    let mut links_guard = self.links.guard();
                    links_guard.clear();

                    if let Some(url) = pkgmodel.url {
                        url.homepage.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Website,
                                link: link,
                            });
                        });
                        url.bugtracker.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::IssueTracker,
                                link: link,
                            });
                        });
                        url.faq.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::FAQ,
                                link: link,
                            });
                        });
                        url.help.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Help,
                                link: link,
                            });
                        });
                        url.donation.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Donate,
                                link: link,
                            });
                        });
                        url.translate.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Translate,
                                link: link,
                            });
                        });
                        url.contact.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Contact,
                                link: link,
                            });
                        });
                        url.vcs_browser.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Source,
                                link: link,
                            });
                        });
                        url.contribute.map(|link| {
                            links_guard.push_back(LinkItemInit {
                                link_type: LinkType::Contribute,
                                link: link,
                            });
                        });
                    }
                }

                {
                    let releases = pkgmodel
                        .releases
                        .iter()
                        .filter(|release| release.release_type == ReleaseType::Stable)
                        .map(|release| {
                            let description = release
                                .description
                                .as_ref()
                                .and_then(|description| description.get("C"))
                                .map(|description| {
                                    let mut input = description.to_string();
                                    // Fix formatting
                                    while input.contains('\n') {
                                        input = input.replace('\n', " ");
                                    }
                                    while input.contains('\t') {
                                        input = input.replace('\t', " ");
                                    }
                                    while input.contains("  ") {
                                        input = input.replace("  ", " ");
                                    }
                                    let mut pango = html2pango::markup_html(&input)
                                        .unwrap_or_else(|_| {
                                            warn!("Pango failed to parse description");
                                            input.to_string()
                                        })
                                        .trim()
                                        .to_string();
                                    while pango.contains("\n ") {
                                        pango = pango.replace("\n ", "\n");
                                    }
                                    while pango.ends_with('\n') {
                                        pango.pop();
                                    }

                                    pango.strip_prefix('\n').unwrap_or(&pango).to_string()
                                });

                            ReleaseItemInit {
                                version: release.version.as_ref().map(|v| v.to_string()),
                                date: release.timestamp.or(release.date),
                                description,
                                url: release.url.as_ref().and_then(|url| url.default.clone()),
                                installed: release.version == self.version,
                            }
                        })
                        .collect::<Vec<_>>();

                    {
                        let mut latest_release_guard = self.latest_release.guard();
                        latest_release_guard.clear();
                        if let Some(release) = releases.get(0) {
                            latest_release_guard.push_back(release.clone());
                        }
                    }

                    if releases.is_empty() {
                        self.set_releases_dialog(None);
                    } else {
                        self.set_releases_dialog(Some(
                            ReleasesDialog::builder().launch(ReleasesInit { releases: releases }),
                        ));
                    }
                }

                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static("image/*"),
                );
                let client = reqwest::Client::builder()
                    .default_headers(headers)
                    .user_agent("nix-software-center")
                    .build()
                    .unwrap();

                for (i, url) in pkgmodel.screenshots.into_iter().enumerate() {
                    if let Ok(home) = env::var("HOME") {
                        let cachedir = format!("{}/.cache/nix-software-center", home);
                        let sha = digest(url.to_string());
                        let scrnpath = format!("{}/screenshots/{}", cachedir, sha);
                        let pkg = self.pkg.clone();
                        let client = client.clone();

                        sender.command(move |out, shutdown| {
                            let url = url.clone();
                            let home = home.clone();
                            let scrnpath = scrnpath.clone();
                            let pkg = pkg.clone();
                            shutdown
                                .register(async move {
                                    tokio::time::sleep(Duration::from_millis(5)).await;
                                    if Path::new(&format!("{}.png", scrnpath)).exists() {
                                        out.send(PkgAsyncMsg::LoadScreenshot(pkg, i, format!("{}.png", scrnpath)));
                                    } else {
                                        match client.get(&url).send().await {
                                            Ok(response) => {
                                                if response.status().is_success() {
                                                    if !Path::new(&format!(
                                                        "{}/.cache/nix-software-center/screenshots",
                                                        home
                                                    ))
                                                    .exists()
                                                    {
                                                        match fs::create_dir_all(format!(
                                                            "{}/.cache/nix-software-center/screenshots",
                                                            home
                                                        )) {
                                                            Ok(_) => {}
                                                            Err(_) => {
                                                                out.send(PkgAsyncMsg::SetError(pkg, i));
                                                                return;
                                                            }
                                                        }
                                                    }
                                                    if let Ok(mut file) = File::create(&scrnpath) {
                                                        if let Ok(b) = response.bytes().await {
                                                            let mut content =  Cursor::new(b);
                                                            if std::io::copy(&mut content, &mut file).is_ok() {
                                                                fn openimg(scrnpath: &str) -> Result<(), Box<dyn Error>> {
                                                                    let img = if let Ok(x) = image::load(BufReader::new(File::open(scrnpath)?), image::ImageFormat::Png) {
                                                                        x
                                                                    } else if let Ok(x) = image::load(BufReader::new(File::open(scrnpath)?), image::ImageFormat::Jpeg) {
                                                                        x
                                                                    } else if let Ok(x) = image::load(BufReader::new(File::open(scrnpath)?), image::ImageFormat::WebP) {
                                                                        x
                                                                    } else {
                                                                        let imgdata = BufReader::new(File::open(scrnpath)?);
                                                                        let format = image::guess_format(imgdata.buffer())?;
                                                                        image::load(imgdata, format)?
                                                                    };
                                                                    let scaled = img.resize(640, 360, FilterType::Lanczos3);
                                                                    let mut output = File::create(format!("{}.png", scrnpath))?;
                                                                    scaled.write_to(&mut output, ImageFormat::Png)?;
                                                                    if let Err(e) = fs::remove_file(scrnpath) {
                                                                        warn!("{}", e);
                                                                    }
                                                                    Ok(())
                                                                }

                                                                match openimg(&scrnpath) {
                                                                    Ok(_) => {
                                                                        out.send(PkgAsyncMsg::LoadScreenshot(
                                                                            pkg, i, format!("{}.png", scrnpath),
                                                                        ));
                                                                    }
                                                                    Err(_) => {
                                                                        if let Err(e) = fs::remove_file(&scrnpath) {
                                                                            warn!("{}", e);
                                                                        }
                                                                        out.send(PkgAsyncMsg::SetError(pkg, i));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        out.send(PkgAsyncMsg::SetError(pkg, i));
                                                        warn!("Error: {}", response.status());
                                                    }
                                                } else {
                                                    out.send(PkgAsyncMsg::SetError(pkg, i));
                                                    warn!("Error: {}", response.status());
                                                }
                                            }
                                            Err(e) => {
                                                out.send(PkgAsyncMsg::SetError(pkg, i));
                                                warn!("Error: {}", e);
                                            }
                                        }
                                    }
                                })
                                .drop_on_shutdown()
                        })
                    }
                }
            }
            PkgMsg::LoadScreenshot(pkg, i, u) => {
                info!("PkgMsg::LoadScreenshot {}", u);
                if pkg == self.pkg {
                    let mut scrn_guard = self.screenshots.guard();
                    if let Some(scrn_widget) = scrn_guard.get_mut(i) {
                        scrn_widget.path = Some(u);
                        trace!("GOT PATH")
                    } else {
                        trace!("NO SCRN WIDGET")
                    }
                } else {
                    trace!("WRONG PACKAGE")
                }
            }
            PkgMsg::SetError(pkg, i) => {
                if pkg == self.pkg {
                    let mut scrn_guard = self.screenshots.guard();
                    if let Some(scrn_widget) = scrn_guard.get_mut(i) {
                        scrn_widget.error = true;
                    }
                }
            }
            PkgMsg::SetCarouselPage(page) => {
                self.carpage = page;
            }
            PkgMsg::OpenHomepage => {
                if let Some(u) = &self.homepage
                    && let Err(e) =
                        gio::AppInfo::launch_default_for_uri(u, gio::AppLaunchContext::NONE)
                {
                    warn!("error: {}", e);
                }
            }
            PkgMsg::Install => {
                let online = checkonline();
                if !online {
                    let _ = sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                let w = WorkPkg {
                    pkg: self.pkg.to_string(),
                    pname: self.pname.to_string(),
                    pkgtype: self.installtype.clone(),
                    action: PkgAction::Install,
                    block: false,
                    notify: None,
                };
                self.workqueue.insert(w.clone());
                if self.workqueue.len() == 1 {
                    self.installworker.emit(InstallAsyncHandlerMsg::Process(w));
                }
            }
            PkgMsg::Remove => {
                let w = WorkPkg {
                    pkg: self.pkg.to_string(),
                    pname: self.pname.to_string(),
                    pkgtype: self.installtype.clone(),
                    action: PkgAction::Remove,
                    block: false,
                    notify: None,
                };
                self.workqueue.insert(w.clone());
                if self.workqueue.len() == 1 {
                    self.installworker.emit(InstallAsyncHandlerMsg::Process(w));
                }
            }
            PkgMsg::FinishedProcess(work) => {
                let _ = nix_data_xinux::utils::refreshicons();
                self.workqueue.remove(&work);
                trace!("WORK QUEUE: {}", self.workqueue.len());
                match work.action {
                    PkgAction::Install => {
                        match work.pkgtype {
                            InstallType::System => {
                                self.installeduserpkgs.insert(work.pkg.to_string())
                            }
                            InstallType::User => self.installedsystempkgs.insert(work.pkg.clone()),
                        };
                        self.installed_pkgs.insert(work.pkg.to_string());
                        if self.launchable.is_none()
                            && let Ok(o) =
                                Command::new("command").arg("-v").arg(&self.pname).output()
                            && o.status.success()
                        {
                            self.set_launchable(Some(Launch::TerminalApp(self.pname.to_string())))
                        }
                    }
                    PkgAction::Remove => {
                        match work.pkgtype {
                            InstallType::System => self.installedsystempkgs.remove(&work.pkg),
                            InstallType::User => self.installeduserpkgs.remove(&work.pkg),
                        };
                        self.installed_pkgs.remove(&work.pkg);
                    }
                }
                let _ = sender.output(AppMsg::UpdateInstalledPkgs);
                if let Some(n) = &work.notify {
                    match n {
                        NotifyPage::Installed => {
                            let _ = sender.output(AppMsg::RemoveInstalledBusy(work));
                        }
                    }
                }

                if !self.workqueue.is_empty()
                    && let Some(w) = self.workqueue.clone().iter().next()
                {
                    self.installworker
                        .emit(InstallAsyncHandlerMsg::Process(w.clone()));
                }
            }
            PkgMsg::FailedProcess(work) => {
                self.workqueue.remove(&work);
                if let Some(n) = &work.notify {
                    match n {
                        NotifyPage::Installed => {
                            let _ = sender.output(AppMsg::RemoveInstalledBusy(work));
                        }
                    }
                }
                if !self.workqueue.is_empty()
                    && let Some(w) = self.workqueue.clone().iter().next()
                {
                    self.installworker
                        .emit(InstallAsyncHandlerMsg::Process(w.clone()));
                }
            }
            PkgMsg::Cancel => {
                // If running, cancel the current process
                if let Some(h) = self.workqueue.iter().next()
                    && h.pkg == self.pkg
                {
                    self.installworker
                        .emit(InstallAsyncHandlerMsg::CancelProcess);
                    return;
                }

                // If not running, remove from queue
                for w in self.workqueue.clone() {
                    if w.pkg == self.pkg {
                        self.workqueue.remove(&w);
                    }
                }
            }
            PkgMsg::CancelFinished => {
                // If running, cancel the current process
                if let Some(h) = self.workqueue.clone().iter().next()
                    && h.pkg == self.pkg
                {
                    self.workqueue.remove(h);
                    return;
                }

                // If not running, remove from queue
                for w in self.workqueue.clone() {
                    if w.pkg == self.pkg {
                        self.workqueue.remove(&w);
                    }
                }
            }
            PkgMsg::Launch => {
                if let Some(l) = &self.launchable {
                    match l {
                        Launch::GtkApp(x) => {
                            let _ = Command::new("gtk-launch").arg(x).spawn();
                        }
                        Launch::TerminalApp(x) => {
                            let _ = Command::new("kgx").arg("-e").arg(x).spawn();
                        }
                    }
                }
            }
            PkgMsg::NixRun => {
                if let Some(l) = &self.launchable {
                    match l {
                        Launch::GtkApp(x) => {
                            debug!("Launching {} with nix shell", x);
                            let _ = Command::new("nix")
                                    .arg("shell")
                                    .arg(format!("nixpkgs#{}", self.pkg))
                                    .arg("--command")
                                    .arg("bash")
                                    .arg("-c")
                                    .arg(format!("env XDG_DATA_DIRS=$XDG_DATA_DIRS:$(nix eval nixpkgs#{}.outPath --raw)/share gtk-launch {}", self.pkg, x))
                                    .spawn();
                        }
                        Launch::TerminalApp(x) => {
                            let cmd = format!(
                                "nix shell nixpkgs#{} --command bash -c \"{}; $SHELL\"",
                                self.pkg, x
                            );
                            launchterm(&cmd);
                        }
                    }
                } else {
                    let cmd = format!(
                        "nix shell nixpkgs#{} --command bash -c \"{}; $SHELL\"",
                        self.pkg, self.pname
                    );
                    launchterm(&cmd);
                }
            }
            PkgMsg::NixShell => {
                let cmd = format!("nix shell nixpkgs#{}", self.pkg);
                launchterm(&cmd);
            }
            PkgMsg::SetInstallType(t) => {
                self.set_installed_pkgs(match t {
                    InstallType::System => self.installedsystempkgs.clone(),
                    InstallType::User => self.installeduserpkgs.clone(),
                });
                self.set_installtype(t.clone());
                let _ = state::update_state(|state| state.install_type = Some(t));
            }
            PkgMsg::AddToQueue(work) => {
                self.workqueue.insert(work.clone());
                if self.workqueue.len() == 1 {
                    self.installworker
                        .emit(InstallAsyncHandlerMsg::Process(work));
                }
            }
            PkgMsg::UpdateOnline(online) => {
                self.set_online(online);
            }
            PkgMsg::ShowReleases => {
                self.releases_dialog
                    .as_ref()
                    .map(|dialog| dialog.widget().present(Some(root)));
            }
            PkgMsg::Noop => (),
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            PkgAsyncMsg::LoadScreenshot(pkg, i, u) => {
                sender.input(PkgMsg::LoadScreenshot(pkg, i, u));
            }
            PkgAsyncMsg::SetError(pkg, i) => {
                sender.input(PkgMsg::SetError(pkg, i));
            }
        }
    }
}

fn launchterm(cmd: &str) {
    let _ = Command::new("kgx").arg("-e").arg(cmd).spawn();
}

relm4::new_action_group!(ModeActionGroup, "mode");
relm4::new_stateless_action!(NixProfileAction, ModeActionGroup, "profile");
relm4::new_stateless_action!(NixSystemAction, ModeActionGroup, "system");

relm4::new_action_group!(RunActionGroup, "run");
relm4::new_stateless_action!(LaunchAction, RunActionGroup, "launch");
relm4::new_stateless_action!(TermShellAction, RunActionGroup, "term");
