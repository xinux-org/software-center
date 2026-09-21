use anyhow::Result;
use gettextrs::gettext;
use html2pango;
use image::{ImageFormat, imageops::FilterType};
use log::*;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{
    WorkerController,
    actions::{RelmAction, RelmActionGroup},
    adw::{self, prelude::*},
    component::Connector,
    factory::FactoryVecDeque,
    gtk::{self, pango},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use spdx::Expression;
use sqlx::SqlitePool;
use std::{
    collections::HashSet,
    convert::identity,
    env,
    error::Error,
    fs::{self, File},
    io::{BufReader, Cursor},
    path::Path,
    process::Command,
    time::Duration,
};

use crate::{
    APPINFO,
    ui::{
        installed::{
            components::installed_item::InstalledItem,
            install_worker::{
                InstallAsyncHandler, InstallAsyncHandlerInit, InstallAsyncHandlerMsg,
            },
        },
        window::{AppMsg, INSTALLED_PACKAGES_STATE, NIX_DATA_CONFIG_STATE, SystemPkgs},
        windowloading::{APPSTREAM_DATA_STATE, PACKAGES_DB_STATE},
    },
    utils::{
        online::{checkonline, checkonline_async},
        packages::{AppData, LicenseEnum, PkgMaintainer, Platform},
        state,
    },
};

use super::components::{
    link_item::{LinkItem, LinkItemInit, LinkItemMsg, LinkType},
    release_item::{ReleaseItem, ReleaseItemInit},
    releases_dialog::{ReleasesDialog, ReleasesInit},
    screenshot::ScreenshotItem,
};

#[tracker::track]
#[derive(Debug)]
pub struct PackagePageModel {
    config: NixDataConfig,

    name: String,
    package: String,
    package_name: String,
    summary: Option<String>,
    description: Option<String>,
    version: Option<String>,

    #[tracker::no_eq]
    urls: Vec<LinkItemInit>,
    screenshot_urls: Vec<String>,

    position: String,

    broken: bool,
    insecure: bool,
    unsupported: bool,
    unfree: bool,

    launchable: Option<Launch>,

    system_package_type: SystemPkgs,

    #[tracker::no_eq]
    icon: gtk::Image,
    #[tracker::no_eq]
    screenshots: FactoryVecDeque<ScreenshotItem>,
    #[tracker::no_eq]
    links: FactoryVecDeque<LinkItem>,
    #[tracker::no_eq]
    latest_release: FactoryVecDeque<ReleaseItem>,
    #[tracker::no_eq]
    install_worker: WorkerController<InstallAsyncHandler>,

    #[tracker::no_eq]
    releases_dialog: Option<Connector<ReleasesDialog>>,

    toast_overlay: adw::ToastOverlay,

    carousel_page: CarouselPage,
    install_type: InstallType,
    installed_packages: HashSet<String>,
    installed_user_packages: HashSet<String>,
    installed_system_packages: HashSet<String>,

    work_queue: HashSet<WorkPackage>,
    online: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct WorkPackage {
    pub package: String,
    pub package_name: String,
    pub install_type: InstallType,
    pub action: PackageAction,
    pub block: bool,
    pub notify: Option<NotifyPage>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum NotifyPage {
    Installed,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PackageAction {
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
    pub spdx_id: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug)]
pub enum PackageMessage {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs),
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    UpdateAppData(Option<AppData>),
    LoadScreenshot(String, usize, String),
    SetError(String, usize),
    SetCarouselPage(CarouselPage),
    Install,
    Remove,
    Cancel,
    CancelFinished,
    FinishedProcess(WorkPackage),
    FailedProcess(WorkPackage),
    Launch,
    NixRun,
    NixShell,
    SetInstallType(InstallType),
    AddToQueue(WorkPackage),
    UpdateOnline(bool),
    ShowReleases,
    ShowToast(String),
    Noop,
}

#[derive(Debug)]
pub enum PackageAsyncMessage {
    LoadScreenshot(String, usize, String),
    SetError(String, usize),
}

#[derive(Debug)]
pub struct PackagePageInit {
    pub package: String,
    pub syspkgs: SystemPkgs,
}

#[relm4::component(pub, async)]
impl AsyncComponent for PackagePageModel {
    type Init = PackagePageInit;
    type Input = PackageMessage;
    type Output = AppMsg;
    type CommandOutput = PackageAsyncMessage;

    view! {
        #[root]
        #[name(package_window)]
        adw::NavigationPage {
            set_title: &model.name,

            adw::BreakpointBin {
                set_width_request: 346,

                add_breakpoint = adw::Breakpoint::new(
                    adw::BreakpointCondition::new_length(
                        adw::BreakpointConditionLengthType::MaxWidth,
                        500.0,
                        adw::LengthUnit::Px,
                    )
                ) {
                    add_setters: &[
                        (&install_options_large, "visible", false),
                    ],
                    add_setters: &[
                        (&install_options_small, "visible", true),
                    ],
                    add_setters: &[
                        (package_icon, "pixel_size", 96)
                    ]
                },

                add_breakpoint = adw::Breakpoint::new(
                    adw::BreakpointCondition::new_length(
                        adw::BreakpointConditionLengthType::MinWidth,
                        500.0,
                        adw::LengthUnit::Px,
                    )
                ) {
                    add_setters: &[
                        (&install_options_large, "visible", true),
                    ],
                    add_setters: &[
                        (&install_options_small, "visible", false),
                    ],
                    add_setters: &[
                        (package_icon, "pixel_size", 128)
                    ]
                },

                #[wrap(Some)]
                #[local_ref]
                set_child = toast_overlay -> adw::ToastOverlay {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        adw::HeaderBar {
                            #[name = "install_type_button"]
                            pack_end = &gtk::MenuButton {
                                #[watch]
                                set_visible: model.system_package_type != SystemPkgs::None,

                                #[watch]
                                set_label: &match model.install_type {
                                    InstallType::User =>  gettext("User (nix profile)"),
                                    InstallType::System => gettext("System (configuration.nix)"),
                                },

                                #[wrap(Some)]
                                set_popover = &gtk::PopoverMenu::from_model(Some(&installtype)) {}
                            }
                        },
                        #[name = "content"]
                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            set_hexpand: true,
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,
                            set_vadjustment: gtk::Adjustment::NONE,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                adw::Clamp {
                                    set_maximum_size: 1000,
                                    set_halign: gtk::Align::Fill,
                                    set_valign: gtk::Align::Start,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_horizontal: 10,
                                        set_margin_bottom: 20,
                                        // Details box
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_spacing: 10,
                                            set_margin_all: 15,
                                            #[local_ref]
                                            append = package_icon -> gtk::Image {
                                                add_css_class: "icon-dropshadow",
                                                set_halign: gtk::Align::Start,
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
                                                            set_lines: 2,
                                                            set_ellipsize: pango::EllipsizeMode::End,
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
                                                            set_lines: 2,
                                                            set_ellipsize: pango::EllipsizeMode::End,
                                                            set_wrap: true,
                                                            set_wrap_mode: pango::WrapMode::WordChar,
                                                            set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                                            #[watch]
                                                            set_label: &model.package,
                                                        },
                                                        gtk::Label {
                                                            add_css_class: "dim-label",
                                                            set_halign: gtk::Align::Start,
                                                            set_ellipsize: pango::EllipsizeMode::End,
                                                            set_wrap: true,
                                                            set_wrap_mode: pango::WrapMode::WordChar,
                                                            set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                                                            #[watch]
                                                            set_label: &model.version.clone().unwrap_or_else(||  gettext("Unknown").to_string()),
                                                        },
                                                    },
                                                },

                                                // Install options
                                                #[name(install_options_large)]
                                                append = &gtk::FlowBoxChild {
                                                    set_halign: gtk::Align::End,
                                                    gtk::Box {
                                                        set_halign: gtk::Align::End,
                                                        set_spacing: 5,
                                                        #[name(install_stack)]
                                                        if model.work_queue.iter().any(|x| x.package == model.package && x.install_type == model.install_type) {
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
                                                                        sender.input(PackageMessage::Cancel)
                                                                    },
                                                                }
                                                            }
                                                        } else if model.installed_packages.contains(&model.package) {
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
                                                                        sender.input(PackageMessage::Launch)
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
                                                                        sender.input(PackageMessage::Remove)
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
                                                                        sender.input(PackageMessage::Install);
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
                                        },

                                        gtk::ScrolledWindow {
                                            set_vscrollbar_policy: gtk::PolicyType::Never,
                                            gtk::Box {
                                                set_spacing: 12,
                                                set_homogeneous: true,
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_spacing: 8,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "round",
                                                        #[watch]
                                                        set_class_active: ("warning", model.unfree),
                                                        gtk::Box {
                                                            set_spacing: 6,
                                                            set_margin_horizontal: 2,
                                                            set_halign: gtk::Align::Center,
                                                            gtk::Image {
                                                                set_icon_name: Some("people-symbolic"),
                                                                #[watch]
                                                                set_visible: !model.unfree,
                                                            },
                                                            gtk::Image {
                                                                set_icon_name: Some("license-symbolic"),
                                                            },
                                                            gtk::Image {
                                                                set_icon_name: Some("nsc-proprietary-code-symbolic"),
                                                                #[watch]
                                                                set_visible: model.unfree,
                                                            },
                                                        },
                                                    },
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &if model.unfree {gettext("Proprietary")} else {gettext("Free Software")},
                                                    },
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_spacing: 8,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "round",
                                                        #[watch]
                                                        set_class_active: ("error", model.broken),
                                                        gtk::Box {
                                                            set_spacing: 6,
                                                            set_margin_horizontal: 2,
                                                            set_halign: gtk::Align::Center,
                                                            gtk::Image {
                                                                set_icon_name: Some("issue-symbolic"),
                                                                #[watch]
                                                                set_visible: !model.broken,
                                                            },
                                                            gtk::Image {
                                                                set_icon_name: Some("issue-symbolic"),
                                                                #[watch]
                                                                set_visible: model.broken,
                                                            },
                                                        },
                                                    },
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &if model.broken {gettext("Broken")} else {gettext("Not Broken")},
                                                    },
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_spacing: 8,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "round",
                                                        #[watch]
                                                        set_class_active: ("error", model.insecure),
                                                        gtk::Box {
                                                            set_spacing: 6,
                                                            set_margin_horizontal: 2,
                                                            set_halign: gtk::Align::Center,
                                                            gtk::Image {
                                                                set_icon_name: Some("shield-safe-symbolic"),
                                                                #[watch]
                                                                set_visible: !model.insecure,
                                                            },
                                                            gtk::Image {
                                                                set_icon_name: Some("shield-danger-symbolic"),
                                                                #[watch]
                                                                set_visible: model.insecure,
                                                            },
                                                        },
                                                    },
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &if model.insecure {gettext("Insecure")} else {gettext("Secure")},
                                                    },
                                                },
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_spacing: 8,
                                                    gtk::Button {
                                                        set_halign: gtk::Align::Center,
                                                        add_css_class: "round",
                                                        #[watch]
                                                        set_class_active: ("error", model.unsupported),
                                                        gtk::Box {
                                                            set_spacing: 6,
                                                            set_margin_horizontal: 2,
                                                            set_halign: gtk::Align::Center,
                                                            gtk::Image {
                                                                set_icon_name: Some("checkmark-small-symbolic"),
                                                                #[watch]
                                                                set_visible: !model.unsupported,
                                                            },
                                                            gtk::Image {
                                                                set_icon_name: Some("cross-small-symbolic"),
                                                                #[watch]
                                                                set_visible: model.unsupported,
                                                            },
                                                        },
                                                    },
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &if model.unsupported {gettext("Unsupported")} else {gettext("Supported")},
                                                    },
                                                },
                                            },
                                        },

                                        #[name(install_options_small)]
                                        gtk::Box {
                                            set_margin_top: 20,
                                            if model.work_queue.iter().any(|x| x.package == model.package && x.install_type == model.install_type) {
                                                gtk::Box {
                                                    set_spacing: 8,
                                                    set_homogeneous: true,
                                                    gtk::Button {
                                                        add_css_class: "pill",
                                                        set_hexpand: true,
                                                        set_sensitive: false,
                                                        set_label: &gettext("Installing..."),
                                                    },
                                                    gtk::Button {
                                                        add_css_class: "pill",
                                                        add_css_class: "destructive-action",
                                                        set_hexpand: true,
                                                        set_label: &gettext("Cancel"),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PackageMessage::Cancel)
                                                        },
                                                    }
                                                }
                                            } else if model.installed_packages.contains(&model.package) {
                                                gtk::Box {
                                                    set_spacing: 8,
                                                    set_homogeneous: true,
                                                    gtk::Button {
                                                        #[watch]
                                                        add_css_class: "pill",
                                                        set_hexpand: true,
                                                        #[watch]
                                                        set_label: &if model.launchable.is_some() { gettext("Open") } else { gettext("Installed") },
                                                        #[watch]
                                                        set_sensitive: model.launchable.is_some(),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PackageMessage::Launch)
                                                        }
                                                    },
                                                    gtk::Button {
                                                        add_css_class: "pill",
                                                        add_css_class: "destructive-action",
                                                        set_hexpand: true,
                                                        set_label: &gettext("Delete"),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PackageMessage::Remove)
                                                        }
                                                    },
                                                }
                                            } else if !model.online {
                                                gtk::Box {
                                                    set_spacing: 8,
                                                    set_homogeneous: true,
                                                    gtk::Button {
                                                        add_css_class: "pill",
                                                        add_css_class: "error",
                                                        set_hexpand: true,
                                                        set_can_target: false,
                                                        set_can_focus: false,
                                                        set_label: &gettext("Offline"),
                                                    },
                                                    gtk::Button {
                                                        add_css_class: "pill",
                                                        set_hexpand: true,
                                                        set_icon_name: "nsc-refresh-symbolic",
                                                        connect_clicked[sender] => move |_| {
                                                            let _ = sender.output(AppMsg::CheckNetwork);
                                                        }
                                                    }
                                                }
                                            } else {
                                                gtk::Box {
                                                    set_spacing: 8,
                                                    set_homogeneous: true,
                                                    gtk::Button {
                                                        #[watch]
                                                        add_css_class: "pill",
                                                        add_css_class: "suggested-action",
                                                        set_hexpand: true,
                                                        set_label: &gettext("Install"),
                                                        connect_clicked[sender] => move |_| {
                                                            sender.input(PackageMessage::Install);
                                                        },
                                                    },
                                                    gtk::MenuButton {
                                                        add_css_class: "pill",
                                                        set_hexpand: true,
                                                        set_label: &gettext("More"),
                                                        #[wrap(Some)]
                                                        set_popover = &gtk::PopoverMenu::from_model(Some(&runaction)) {},
                                                    },
                                                }
                                            },
                                        },
                                    },

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
                                                    sender.input(PackageMessage::SetCarouselPage(CarouselPage::Single));
                                                } else if i == 0 {
                                                    sender.input(PackageMessage::SetCarouselPage(CarouselPage::First));
                                                } else if i == n - 1 {
                                                    sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
                                                } else {
                                                    sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                                                }
                                            },
                                        },
                                        add_overlay = &gtk::Revealer {
                                            set_transition_type: gtk::RevealerTransitionType::Crossfade,
                                            #[watch]
                                            set_reveal_child: model.carousel_page != CarouselPage::First && model.carousel_page != CarouselPage::Single,
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
                                                        sender.input(PackageMessage::SetCarouselPage(CarouselPage::First));
                                                    } else if i > 0 {
                                                        sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                                                    }
                                                }
                                            }
                                        },
                                        add_overlay = &gtk::Revealer {
                                            set_transition_type: gtk::RevealerTransitionType::Crossfade,
                                            #[watch]
                                            set_reveal_child: model.carousel_page != CarouselPage::Last && model.carousel_page != CarouselPage::Single,
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
                                                        sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
                                                    } else if i <= n - 2 {
                                                        sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                                                    } else {
                                                        sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
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
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 15,
                                        set_margin_horizontal: 10,
                                        set_margin_vertical: 30,

                                        #[watch]
                                        set_visible: !(model.summary.is_none() && model.description.is_none()),
                                        gtk::Box {
                                            set_vexpand: true,
                                            set_valign: gtk::Align::Start,
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 10,
                                            gtk::Label {
                                                add_css_class: "title-2",
                                                set_valign: gtk::Align::Start,
                                                set_halign: gtk::Align::Start,
                                                set_selectable: true,
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
                                                set_selectable: true,
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

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 12,
                                            #[watch]
                                            set_visible: model.releases_dialog.is_some(),
                                            #[local_ref]
                                            latest_release_factory -> adw::PreferencesGroup {},
                                            adw::PreferencesGroup {
                                                adw::ButtonRow {
                                                    set_title: &gettext("Version History"),
                                                    set_end_icon_name: Some("right-symbolic"),
                                                    connect_activated[sender] => move |_| {
                                                        sender.input(PackageMessage::ShowReleases);
                                                    }
                                                }
                                            }
                                        },

                                        gtk::Box {
                                            set_vexpand: true,
                                            set_valign: gtk::Align::Start,
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 10,
                                            #[watch]
                                            set_visible: !model.links.is_empty(),
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
                                },
                                gtk::Separator {
                                    set_vexpand: true,
                                    add_css_class: "spacer"
                                }
                            }
                        }
                    },
                },
            },
        },
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

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            PackageMessage::UpdateInstalledPackages {
                system_packages: state.installed_system_packages.clone(),
                user_packages: state.installed_user_packages.clone(),
            }
        });

        NIX_DATA_CONFIG_STATE.subscribe(sender.input_sender(), |state| {
            PackageMessage::UpdateConfig(state.clone())
        });

        let package_ = init.package.clone();
        APPSTREAM_DATA_STATE.subscribe(sender.input_sender(), move |state| {
            let app_data = state
                .iter()
                .find(|(package, _app_data)| package == &&package_)
                .map(|(_package, app_data)| app_data.clone());

            PackageMessage::UpdateAppData(app_data)
        });

        let install_worker = InstallAsyncHandler::builder()
            .detach_worker(InstallAsyncHandlerInit {
                syspkgs: init.syspkgs.clone(),
            })
            .forward(sender.input_sender(), identity);
        let config = NIX_DATA_CONFIG_STATE.read().clone();
        install_worker.emit(InstallAsyncHandlerMsg::SetConfig(config.clone()));

        let online = checkonline_async().await;

        let installed_packages = INSTALLED_PACKAGES_STATE.read();
        let installed_system_packages = installed_packages
            .installed_system_packages
            .iter()
            .map(|item| item.pkg.to_string())
            .collect::<HashSet<_>>();
        let installed_user_packages = installed_packages
            .installed_user_packages
            .iter()
            .map(|item| item.pkg.to_string())
            .collect::<HashSet<_>>();

        let install_type = {
            let is_system_pkg = installed_system_packages.contains(&init.package);
            let is_user_pkg = installed_user_packages.contains(&init.package);

            match (is_system_pkg, is_user_pkg) {
                (true, false) => InstallType::System,
                (false, true) => InstallType::User,
                _ => state::get_state()
                    .and_then(|state| state.install_type)
                    .unwrap_or(InstallType::User),
            }
        };

        let installed_packages = match install_type {
            InstallType::System => installed_system_packages.clone(),
            InstallType::User => installed_user_packages.clone(),
        };

        let mut model = PackagePageModel {
            config,
            name: String::default(),
            package: init.package.clone(),
            package_name: String::default(),
            summary: None,
            description: None,
            version: None,

            urls: vec![],
            screenshot_urls: vec![],

            position: String::default(),

            broken: false,
            insecure: false,
            unsupported: false,
            unfree: false,

            icon: gtk::Image::new(),
            screenshots: FactoryVecDeque::builder()
                .launch(adw::Carousel::new())
                .detach(),
            links: FactoryVecDeque::builder()
                .launch(adw::PreferencesGroup::new())
                .forward(sender.input_sender(), |msg| match msg {
                    LinkItemMsg::ShowToast(msg) => PackageMessage::ShowToast(msg),
                }),
            latest_release: FactoryVecDeque::builder()
                .launch(adw::PreferencesGroup::new())
                .detach(),
            install_worker,

            releases_dialog: None,

            toast_overlay: adw::ToastOverlay::new(),

            carousel_page: CarouselPage::Single,

            install_type,
            installed_packages,
            installed_user_packages,
            installed_system_packages,
            system_package_type: init.syspkgs,

            work_queue: HashSet::new(),
            launchable: None,
            online,
            tracker: 0,
        };

        let package_db = &PACKAGES_DB_STATE.read().packages_db;
        if let Ok(pool) = &SqlitePool::connect(&format!("sqlite://{}", package_db)).await {
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
            .bind(&init.package)
            .fetch_one(pool)
            .await;

            if let Ok((
                package_name,
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
                model.name = package_name.to_string();

                model.summary = if description.is_empty() {
                    None
                } else {
                    Some(description)
                };

                model.description = if longdescription.is_empty() {
                    None
                } else {
                    Some(longdescription)
                };

                model.version = Some(version);

                model.broken = broken;
                model.insecure = insecure;
                model.unsupported = unsupported;
                model.unfree = unfree;

                let mut licenses = vec![];
                let mut platforms = vec![];
                let mut maintainers = vec![];

                let appstream_data_state = APPSTREAM_DATA_STATE.read();

                let app_data = appstream_data_state.get(&init.package);

                if let Some(data) = app_data {
                    apply_appstream_data(&mut model, &sender, data);
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

                if let Ok(m) = serde_json::from_str::<Vec<PkgMaintainer>>(&maintainersjson) {
                    for m in m {
                        maintainers.push(m);
                    }
                }

                {
                    let mut links_guard = model.links.guard();

                    model.urls.iter().for_each(|link| {
                        links_guard.push_back(link.clone());
                    });

                    links_guard.push_back(LinkItemInit {
                        link_type: LinkType::NixSource,
                        link: "https://github.com/NixOS/nixpkgs/blob/nixos-unstable/".to_string()
                            + &position.replace(':', "#L"),
                    });
                }
            }
        } else {
            error!("No pkgdb!");
        }

        let model = model;

        let toast_overlay = &model.toast_overlay;

        let package_icon = &model.icon;

        let link_factory = model.links.widget();

        let latest_release_factory = model.latest_release.widget();

        info!("latest release: {:?}", model.latest_release.get(0));
        info!("latest release factory: {:?}", latest_release_factory);

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

        let mut install_type_group = RelmActionGroup::<ModeActionGroup>::new();

        let nixprofile: RelmAction<NixProfileAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PackageMessage::SetInstallType(InstallType::User));
            })
        };

        let nixsystem: RelmAction<NixSystemAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PackageMessage::SetInstallType(InstallType::System));
            })
        };

        install_type_group.add_action(nixprofile);
        install_type_group.add_action(nixsystem);

        let install_type_actions = install_type_group.into_action_group();
        widgets
            .install_type_button
            .insert_action_group("install_type", Some(&install_type_actions));

        let mut run_group = RelmActionGroup::<RunActionGroup>::new();
        let launch_action: RelmAction<LaunchAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(PackageMessage::NixRun);
            })
        };

        let term_action: RelmAction<TermShellAction> = {
            let sender = sender;
            RelmAction::new_stateless(move |_| sender.input(PackageMessage::NixShell))
        };

        run_group.add_action(launch_action);
        run_group.add_action(term_action);

        let run_actions = run_group.into_action_group();
        widgets
            .content
            .insert_action_group("run", Some(&run_actions));

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.reset();
        match msg {
            PackageMessage::UpdateConfig(config) => {
                self.config = config.clone();
                self.install_worker
                    .emit(InstallAsyncHandlerMsg::SetConfig(config));
            }
            PackageMessage::UpdatePkgTypes(system_package_type) => {
                self.system_package_type = system_package_type.clone();
                self.install_worker
                    .emit(InstallAsyncHandlerMsg::SetPkgTypes(system_package_type));
            }
            PackageMessage::UpdateInstalledPackages {
                system_packages,
                user_packages,
            } => {
                let system_packages = system_packages
                    .iter()
                    .map(|item| item.pkg.to_string())
                    .collect::<HashSet<_>>();
                let user_packages = user_packages
                    .iter()
                    .map(|item| item.pkg.to_string())
                    .collect::<HashSet<_>>();

                self.set_installed_system_packages(system_packages);
                self.set_installed_user_packages(user_packages);
                self.set_installed_packages(match self.install_type {
                    InstallType::System => self.installed_system_packages.clone(),
                    InstallType::User => self.installed_user_packages.clone(),
                });
            }
            PackageMessage::UpdateAppData(app_data) => {
                if let Some(app_data) = app_data {
                    apply_appstream_data(self, &sender, &app_data);
                }
            }
            PackageMessage::LoadScreenshot(pkg, i, u) => {
                info!("PkgMsg::LoadScreenshot {}", u);
                if pkg == self.package {
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
            PackageMessage::SetError(pkg, i) => {
                if pkg == self.package {
                    let mut scrn_guard = self.screenshots.guard();
                    if let Some(scrn_widget) = scrn_guard.get_mut(i) {
                        scrn_widget.error = true;
                    }
                }
            }
            PackageMessage::SetCarouselPage(page) => {
                self.carousel_page = page;
            }
            PackageMessage::Install => {
                let online = checkonline();
                if !online {
                    let _ = sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                let w = WorkPackage {
                    package: self.package.to_string(),
                    package_name: self.package_name.to_string(),
                    install_type: self.install_type.clone(),
                    action: PackageAction::Install,
                    block: false,
                    notify: None,
                };
                self.work_queue.insert(w.clone());
                if self.work_queue.len() == 1 {
                    self.install_worker.emit(InstallAsyncHandlerMsg::Process(w));
                }
            }
            PackageMessage::Remove => {
                let w = WorkPackage {
                    package: self.package.to_string(),
                    package_name: self.package_name.to_string(),
                    install_type: self.install_type.clone(),
                    action: PackageAction::Remove,
                    block: false,
                    notify: None,
                };
                self.work_queue.insert(w.clone());
                if self.work_queue.len() == 1 {
                    self.install_worker.emit(InstallAsyncHandlerMsg::Process(w));
                }
            }
            PackageMessage::FinishedProcess(work) => {
                let _ = nix_data_xinux::utils::refreshicons();
                self.work_queue.remove(&work);
                trace!("WORK QUEUE: {}", self.work_queue.len());
                match work.action {
                    PackageAction::Install => {
                        match work.install_type {
                            InstallType::System => self
                                .installed_user_packages
                                .insert(work.package.to_string()),
                            InstallType::User => {
                                self.installed_system_packages.insert(work.package.clone())
                            }
                        };
                        self.installed_packages.insert(work.package.to_string());
                        if self.launchable.is_none()
                            && let Ok(o) = Command::new("command")
                                .arg("-v")
                                .arg(&self.package_name)
                                .output()
                            && o.status.success()
                        {
                            self.set_launchable(Some(Launch::TerminalApp(
                                self.package_name.to_string(),
                            )))
                        }
                    }
                    PackageAction::Remove => {
                        match work.install_type {
                            InstallType::System => {
                                self.installed_system_packages.remove(&work.package)
                            }
                            InstallType::User => self.installed_user_packages.remove(&work.package),
                        };
                        self.installed_packages.remove(&work.package);
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

                if !self.work_queue.is_empty()
                    && let Some(w) = self.work_queue.clone().iter().next()
                {
                    self.install_worker
                        .emit(InstallAsyncHandlerMsg::Process(w.clone()));
                }
            }
            PackageMessage::FailedProcess(work) => {
                self.work_queue.remove(&work);
                if let Some(n) = &work.notify {
                    match n {
                        NotifyPage::Installed => {
                            let _ = sender.output(AppMsg::RemoveInstalledBusy(work));
                        }
                    }
                }
                if !self.work_queue.is_empty()
                    && let Some(w) = self.work_queue.clone().iter().next()
                {
                    self.install_worker
                        .emit(InstallAsyncHandlerMsg::Process(w.clone()));
                }
            }
            PackageMessage::Cancel => {
                // If running, cancel the current process
                if let Some(h) = self.work_queue.iter().next()
                    && h.package == self.package
                {
                    self.install_worker
                        .emit(InstallAsyncHandlerMsg::CancelProcess);
                    return;
                }

                // If not running, remove from queue
                for w in self.work_queue.clone() {
                    if w.package == self.package {
                        self.work_queue.remove(&w);
                    }
                }
            }
            PackageMessage::CancelFinished => {
                // If running, cancel the current process
                if let Some(h) = self.work_queue.clone().iter().next()
                    && h.package == self.package
                {
                    self.work_queue.remove(h);
                    return;
                }

                // If not running, remove from queue
                for w in self.work_queue.clone() {
                    if w.package == self.package {
                        self.work_queue.remove(&w);
                    }
                }
            }
            PackageMessage::Launch => {
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
            PackageMessage::NixRun => {
                if let Some(l) = &self.launchable {
                    match l {
                        Launch::GtkApp(x) => {
                            debug!("Launching {} with nix shell", x);
                            let _ = Command::new("nix")
                                    .arg("shell")
                                    .arg(format!("nixpkgs#{}", self.package))
                                    .arg("--command")
                                    .arg("bash")
                                    .arg("-c")
                                    .arg(format!("env XDG_DATA_DIRS=$XDG_DATA_DIRS:$(nix eval nixpkgs#{}.outPath --raw)/share gtk-launch {}", self.package, x))
                                    .spawn();
                        }
                        Launch::TerminalApp(x) => {
                            let cmd = format!(
                                "nix shell nixpkgs#{} --command bash -c \"{}; $SHELL\"",
                                self.package, x
                            );
                            launchterm(&cmd);
                        }
                    }
                } else {
                    let cmd = format!(
                        "nix shell nixpkgs#{} --command bash -c \"{}; $SHELL\"",
                        self.package, self.package_name
                    );
                    launchterm(&cmd);
                }
            }
            PackageMessage::NixShell => {
                let cmd = format!("nix shell nixpkgs#{}", self.package);
                launchterm(&cmd);
            }
            PackageMessage::SetInstallType(t) => {
                self.set_installed_packages(match t {
                    InstallType::System => self.installed_system_packages.clone(),
                    InstallType::User => self.installed_user_packages.clone(),
                });
                self.set_install_type(t.clone());
                let _ = state::update_state(|state| state.install_type = Some(t));
            }
            PackageMessage::AddToQueue(work) => {
                self.work_queue.insert(work.clone());
                if self.work_queue.len() == 1 {
                    self.install_worker
                        .emit(InstallAsyncHandlerMsg::Process(work));
                }
            }
            PackageMessage::UpdateOnline(online) => {
                self.set_online(online);
            }
            PackageMessage::ShowReleases => {
                self.releases_dialog
                    .as_ref()
                    .map(|dialog| dialog.widget().present(Some(root)));
            }
            PackageMessage::ShowToast(msg) => {
                let toast = adw::Toast::new(&msg);
                toast.set_timeout(2);
                self.toast_overlay.add_toast(toast);
            }
            PackageMessage::Noop => (),
        }
    }

    async fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            PackageAsyncMessage::LoadScreenshot(pkg, i, u) => {
                sender.input(PackageMessage::LoadScreenshot(pkg, i, u));
            }
            PackageAsyncMessage::SetError(pkg, i) => {
                sender.input(PackageMessage::SetError(pkg, i));
            }
        }
    }
}

fn launchterm(cmd: &str) {
    let _ = Command::new("kgx").arg("-e").arg(cmd).spawn();
}

fn addlicense(pkglicense: &LicenseEnum, licenses: &mut Vec<License>) {
    match pkglicense {
        LicenseEnum::Single(l) => {
            if let Some(n) = &l.fullname {
                let parsed = if let Some(id) = &l.spdxid {
                    if let Ok(Some(license)) = Expression::parse(id).map(|p| {
                        p.requirements()
                            .map(|er| er.req.license.id())
                            .collect::<Vec<_>>()[0]
                    }) {
                        Some(license)
                    } else {
                        None
                    }
                } else if let Ok(Some(license)) = Expression::parse(n).map(|p| {
                    p.requirements()
                        .map(|er| er.req.license.id())
                        .collect::<Vec<_>>()[0]
                }) {
                    Some(license)
                } else {
                    None
                };
                licenses.push(License {
                    free: if let Some(f) = l.free {
                        Some(f)
                    } else {
                        parsed.map(|p| p.is_osi_approved() || p.is_fsf_free_libre())
                    },
                    fullname: n.to_string(),
                    spdx_id: l.spdxid.clone(),
                    url: if let Some(u) = &l.url {
                        Some(u.to_string())
                    } else {
                        parsed.map(|p| format!("https://spdx.org/licenses/{}.html", p.name))
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
                    spdx_id: Some(license.name.to_string()),
                    url: if l.url.is_some() {
                        l.url.clone()
                    } else {
                        Some(format!("https://spdx.org/licenses/{}.html", license.name))
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
                    free: Some(license.is_osi_approved() || license.is_fsf_free_libre()),
                    fullname: license.full_name.to_string(),
                    spdx_id: Some(license.name.to_string()),
                    url: Some(format!("https://spdx.org/licenses/{}.html", license.name)),
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

fn html_to_pango(text: &str) -> String {
    let mut text = text.to_string();

    // Fix formatting
    while text.contains('\n') {
        text = text.replace('\n', " ");
    }
    while text.contains('\t') {
        text = text.replace('\t', " ");
    }
    while text.contains("  ") {
        text = text.replace("  ", " ");
    }

    text = html2pango::markup_html(&text)
        .unwrap_or_else(|_| {
            warn!("Pango failed to parse text: {}", text);
            text
        })
        .trim()
        .to_string();

    while text.contains("\n ") {
        text = text.replace("\n ", "\n");
    }

    while text.ends_with('\n') {
        text.pop();
    }

    text = text.strip_prefix('\n').unwrap_or(&text).to_string();

    text
}

fn apply_appstream_data(
    model: &mut PackagePageModel,
    sender: &AsyncComponentSender<PackagePageModel>,
    data: &AppData,
) {
    if let Some(names) = &data.name
        && let Some(name) = names.get("C")
    {
        model.name = name.clone();
    }

    if let Some(summaries) = &data.summary
        && let Some(summary) = summaries.get("C")
    {
        model.summary = Some(summary.clone());
    }

    if let Some(descriptions) = &data.description
        && let Some(description) = descriptions.get("C")
    {
        model.description = Some(html_to_pango(description));
    }

    model.update_icon(|icon| {
        let app_icon = data
            .icon
            .as_ref()
            .and_then(|icon_list| icon_list.cached.as_ref())
            .and_then(|icons| {
                let mut icons = icons.clone();
                icons.sort_by_key(|icon| icon.height);
                icons.last().cloned()
            });

        icon.clear();
        if let Some(app_icon) = app_icon {
            icon.set_from_file(Some(format!(
                "{}/icons/nixos/{}x{}/{}",
                APPINFO, app_icon.width, app_icon.height, app_icon.name
            )));
        } else {
            icon.set_icon_name(Some("package-x-generic"));
        }
    });

    if let Some(app_screenshots) = &data.screenshots {
        let mut screenshot_urls = vec![];

        for screenshot in app_screenshots {
            if let Some(image) = &screenshot.sourceimage {
                if !screenshot_urls.contains(&image.url) {
                    if screenshot.default.unwrap_or_default() {
                        screenshot_urls.insert(0, image.url.clone());
                    } else {
                        screenshot_urls.push(image.url.clone());
                    }
                } else if screenshot.default.unwrap_or_default()
                    && let Some(index) = screenshot_urls.iter().position(|x| *x == image.url)
                {
                    screenshot_urls.remove(index);
                    screenshot_urls.insert(0, image.url.clone());
                }
            }
        }

        if screenshot_urls.len() <= 1 {
            model.carousel_page = CarouselPage::Single;
        } else {
            model.carousel_page = CarouselPage::First;
        }

        model.screenshot_urls = screenshot_urls.clone();

        model.update_screenshots(|screenshots| {
            let mut guard = screenshots.guard();
            guard.clear();
            for _ in &screenshot_urls {
                guard.push_back(());
            }
        });

        load_screenshots(sender, &model.package, screenshot_urls.clone());
    }

    model.launchable = if let Some(l) = data.launchable.as_ref()
        && let Some(d) = l.desktopid.first()
    {
        Some(Launch::GtkApp(d.clone()))
    } else if model.installed_user_packages.contains(&model.package)
        && let Ok(o) = Command::new("command")
            .arg("-v")
            .arg(&model.package_name)
            .output()
        && o.status.success()
    {
        Some(Launch::TerminalApp(model.package_name.clone()))
    } else {
        None
    };

    let mut urls = vec![];

    if let Some(url) = data.url.clone() {
        if let Some(link) = url.homepage {
            urls.push(LinkItemInit {
                link_type: LinkType::Website,
                link,
            });
        }
        if let Some(link) = url.bugtracker {
            urls.push(LinkItemInit {
                link_type: LinkType::IssueTracker,
                link,
            });
        }
        if let Some(link) = url.faq {
            urls.push(LinkItemInit {
                link_type: LinkType::FAQ,
                link,
            });
        }
        if let Some(link) = url.help {
            urls.push(LinkItemInit {
                link_type: LinkType::Help,
                link,
            });
        }
        if let Some(link) = url.donation {
            urls.push(LinkItemInit {
                link_type: LinkType::Donate,
                link,
            });
        }
        if let Some(link) = url.translate {
            urls.push(LinkItemInit {
                link_type: LinkType::Translate,
                link,
            });
        }
        if let Some(link) = url.contact {
            urls.push(LinkItemInit {
                link_type: LinkType::Contact,
                link,
            });
        }
        if let Some(link) = url.vcs_browser {
            urls.push(LinkItemInit {
                link_type: LinkType::Source,
                link,
            });
        }
        if let Some(link) = url.contribute {
            urls.push(LinkItemInit {
                link_type: LinkType::Contribute,
                link,
            });
        }
    }

    model.urls = urls;

    if let Some(releases) = data.releases.as_ref() {
        let releases = releases
            .iter()
            .map(|release| ReleaseItemInit {
                version: release.version.clone(),
                date: release.date,
                description: release
                    .description
                    .as_ref()
                    .and_then(|descriptions| descriptions.get("C"))
                    .map(|description| html_to_pango(description)),
                url: release.url.as_ref().and_then(|url| url.details.clone()),
                installed: false,
            })
            .collect::<Vec<_>>();

        if let Some(release) = releases.first() {
            model.latest_release =
                FactoryVecDeque::from_iter(vec![release.clone()], adw::PreferencesGroup::new());
        }

        if !releases.is_empty() {
            let connector = ReleasesDialog::builder().launch(ReleasesInit { releases });
            model.releases_dialog = Some(connector);
        }
    }
}

fn load_screenshots(
    sender: &AsyncComponentSender<PackagePageModel>,
    package: &str,
    screenshot_urls: Vec<String>,
) {
    debug!("Loading screenshots for package '{package}'");
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

    if let Ok(home) = env::var("HOME") {
        let cache_dir = format!("{home}/.cache/nix-software-center/screenshots");

        for (i, url) in screenshot_urls.into_iter().enumerate() {
            let sha = digest(&url);

            let client = client.clone();
            let package = package.to_string();
            let cache_dir = cache_dir.clone();

            sender.command(move |output_sender, shutdown| {
                shutdown
                    .register(async move {
                        let path = format!("{cache_dir}/{sha}.png");
                        if Path::new(&path).exists() {
                            output_sender
                                .send(PackageAsyncMessage::LoadScreenshot(package, i, path));
                        } else {
                            if let Ok(path) = load_screenshot(&client, url, cache_dir, sha).await {
                                output_sender
                                    .send(PackageAsyncMessage::LoadScreenshot(package, i, path));
                            } else {
                                output_sender.send(PackageAsyncMessage::SetError(package, i));
                            }
                        }
                    })
                    .drop_on_shutdown()
            });
        }
    }
}

async fn load_screenshot(
    client: &reqwest::Client,
    url: String,
    output_directory: String,
    sha: String,
) -> anyhow::Result<String> {
    debug!("Loading screenshot '{url}'");
    tokio::time::sleep(Duration::from_millis(5)).await;
    let path = format!("{output_directory}/{sha}.png");
    if Path::new(&format!("{path}.png")).exists() {
        Ok(path)
    } else {
        download_screenshot(client, url, output_directory, sha).await?;
        Ok(path)
    }
}

async fn download_screenshot(
    client: &reqwest::Client,
    url: String,
    output_directory: String,
    sha: String,
) -> anyhow::Result<String> {
    debug!("Downloading screenshot '{url}'");
    let path = format!("{output_directory}/{sha}.png");
    let path_temp = format!("{output_directory}/{sha}");

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Screenshot could not be downloaded");
    }

    if !Path::new(&output_directory).exists() {
        fs::create_dir_all(output_directory)?;
    }

    let mut file = File::create(&path_temp)?;
    let bytes = response.bytes().await?;
    let mut content = Cursor::new(bytes);
    std::io::copy(&mut content, &mut file)?;

    normalize_screenshot(&path_temp, &path)?;

    Ok(path)
}

fn normalize_screenshot(old_path: &str, new_path: &str) -> anyhow::Result<()> {
    debug!("Normalizing screenshot '{old_path}'");
    let img = image::load(
        BufReader::new(File::open(old_path)?),
        image::ImageFormat::Png,
    )
    .or_else(|_| {
        image::load(
            BufReader::new(File::open(old_path)?),
            image::ImageFormat::Jpeg,
        )
    })
    .or_else(|_| {
        image::load(
            BufReader::new(File::open(old_path)?),
            image::ImageFormat::WebP,
        )
    })
    .or_else(|_| {
        let image_data = BufReader::new(File::open(old_path)?);
        let format = image::guess_format(image_data.buffer())?;
        image::load(image_data, format)
    })?;

    let scaled = img.resize(640, 360, FilterType::Lanczos3);
    let mut output = File::create(new_path)?;
    scaled.write_to(&mut output, ImageFormat::Png)?;

    if let Err(e) = fs::remove_file(old_path) {
        warn!("Could not delete file {}: {}", old_path, e);
    }

    Ok(())
}

relm4::new_action_group!(ModeActionGroup, "install_type");
relm4::new_stateless_action!(NixProfileAction, ModeActionGroup, "profile");
relm4::new_stateless_action!(NixSystemAction, ModeActionGroup, "system");

relm4::new_action_group!(RunActionGroup, "run");
relm4::new_stateless_action!(LaunchAction, RunActionGroup, "launch");
relm4::new_stateless_action!(TermShellAction, RunActionGroup, "term");
