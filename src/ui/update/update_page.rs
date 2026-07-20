use adw::prelude::*;
use gettextrs::gettext;
use log::*;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{factory::*, *};
use std::{collections::HashMap, convert::identity};

use crate::{
    ui::{
        package::package_page::InstallType,
        rebuild::rebuild_model::RebuildMsg,
        update::{
            components::update_item::{UpdateItem, UpdateItemModel},
            unavailable_dialog::{UnavailableDialogModel, UnavailableDialogMsg},
            update_worker::{UpdateAsyncHandler, UpdateAsyncHandlerInit, UpdateAsyncHandlerMsg},
        },
        window::*,
    },
    utils::online::checkonline,
};

pub static UNAVAILABLE_BROKER: MessageBroker<UnavailableDialogMsg> = MessageBroker::new();

#[tracker::track]
#[derive(Debug)]
pub struct UpdatePageModel {
    #[tracker::no_eq]
    updateuserlist: FactoryVecDeque<UpdateItemModel>,
    #[tracker::no_eq]
    updatesystemlist: FactoryVecDeque<UpdateItemModel>,
    channelupdate: Option<(String, String)>,
    #[tracker::no_eq]
    updateworker: WorkerController<UpdateAsyncHandler>,
    config: NixDataConfig,
    systype: SystemPkgs,
    updatetracker: u8,
    #[tracker::no_eq]
    unavailabledialog: Controller<UnavailableDialogModel>,
    online: bool,
}

#[derive(Debug)]
pub enum UpdatePageMsg {
    UpdateConfig(NixDataConfig),
    UpdatePkgTypes(SystemPkgs),
    Update(Vec<UpdateItem>, Vec<UpdateItem>),
    OpenRow(usize, InstallType),
    UpdateSystem,
    UpdateSystemRm(Vec<String>),
    UpdateAllUser,
    UpdateAllUserRm(Vec<String>),
    UpdateUser(String),
    // UpdateChannels,
    // UpdateSystemAndChannels,
    UpdateAll,
    UpdateAllRm(Vec<String>, Vec<String>),
    DoneWorking,
    FailedWorking,
    UpdateOnline(bool),
    Noop,
}

#[derive(Debug)]
pub enum UpdateType {
    System,
    User,
    All,
}

pub struct UpdatePageInit {
    pub window: gtk::Window,
    pub systype: SystemPkgs,
    pub config: NixDataConfig,
    pub online: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for UpdatePageModel {
    type Init = UpdatePageInit;
    type Input = UpdatePageMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(UpdatePageModel::updatetracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            adw::Clamp {
                #[name(mainstack)]
                if !model.online {
                    adw::StatusPage {
                        set_icon_name: Some("nsc-network-offline-symbolic"),
                        set_title: &gettext("No internet connection"),
                        set_description: Some(&gettext("Please connect to the internet to update your system")),
                        gtk::Button {
                            add_css_class: "pill",
                            set_halign: gtk::Align::Center,
                            adw::ButtonContent {
                                set_icon_name: "nsc-refresh-symbolic",
                                set_label: &gettext("Refresh"),
                            },
                            connect_clicked[sender] => move |_| {
                                let _ = sender.output(AppMsg::CheckNetwork);
                            }
                        }
                    }
                } else if !model.updateuserlist.is_empty() || !model.updatesystemlist.is_empty() {
                    // model.channelupdate.is_some() ||
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Start,
                        set_margin_all: 15,
                        set_spacing: 15,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-2",
                                set_label: &gettext("Updates"),
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: &gettext("Update Everything"),
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAll);
                                }
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: &gettext("User (nix profile)")
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: &gettext("Update All"),
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAllUser);
                                }
                            }
                        },
                        #[local_ref]
                        updateuserlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(UpdatePageMsg::OpenRow(i as usize, InstallType::User));
                                }
                            },
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.updatesystemlist.is_empty(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: &gettext("System (configuration.nix)"),
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                                set_label: &gettext("Update"),
                                connect_clicked[sender] => move |_|{
                                    sender.input(UpdatePageMsg::UpdateSystem);
                                },
                            }
                        },
                        #[local_ref]
                        updatesystemlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(UpdatePageMsg::OpenRow(i as usize, InstallType::System));
                                }
                            },
                            #[watch]
                            set_visible: !model.updatesystemlist.is_empty(),
                        }
                    }
                } else {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 10,
                        gtk::Image {
                            add_css_class: "success",
                            set_icon_name: Some("emblem-ok-symbolic"),
                            set_pixel_size: 256,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: &gettext("Everything is up to date!")
                        }
                    }
                }
            }
        }
    }

    fn init(
        initparams: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let updateworker = UpdateAsyncHandler::builder()
            .detach_worker(UpdateAsyncHandlerInit {
                syspkgs: initparams.systype.clone(),
            })
            .forward(sender.input_sender(), identity);

        let unavailabledialog = UnavailableDialogModel::builder()
            .launch_with_broker(initparams.window.clone(), &UNAVAILABLE_BROKER)
            .forward(sender.input_sender(), identity);

        let config = initparams.config;
        updateworker.emit(UpdateAsyncHandlerMsg::UpdateConfig(config.clone()));

        let model = UpdatePageModel {
            updateuserlist: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .forward(sender.input_sender(), |_| UpdatePageMsg::Noop),
            updatesystemlist: FactoryVecDeque::builder()
                .launch(gtk::ListBox::new())
                .forward(sender.input_sender(), |_| UpdatePageMsg::Noop),
            channelupdate: None,
            updatetracker: 0,
            updateworker,
            config,
            systype: initparams.systype,
            unavailabledialog,
            online: initparams.online,
            tracker: 0,
        };

        let updateuserlist = model.updateuserlist.widget();
        let updatesystemlist = model.updatesystemlist.widget();

        let widgets = view_output!();
        widgets.mainstack.set_hhomogeneous(false);
        widgets.mainstack.set_vhomogeneous(false);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            UpdatePageMsg::UpdateConfig(config) => {
                self.config = config;
                self.updateworker
                    .emit(UpdateAsyncHandlerMsg::UpdateConfig(self.config.clone()));
            }
            UpdatePageMsg::UpdatePkgTypes(systype) => {
                self.systype = systype;
                self.updateworker
                    .emit(UpdateAsyncHandlerMsg::UpdatePkgTypes(self.systype.clone()));
            }
            UpdatePageMsg::Update(updateuserlist, updatesystemlist) => {
                info!("UpdatePageMsg::Update");
                debug!("UPDATEUSERLIST: {:?}", updateuserlist);
                debug!("UPDATESYSTEMLIST: {:?}", updatesystemlist);
                self.update_updatetracker(|_| ());
                let mut updateuserlist_guard = self.updateuserlist.guard();
                updateuserlist_guard.clear();
                for updateuser in updateuserlist {
                    updateuserlist_guard.push_back(updateuser);
                }
                let mut updatesystemlist_guard = self.updatesystemlist.guard();
                updatesystemlist_guard.clear();
                for updatesystem in updatesystemlist {
                    updatesystemlist_guard.push_back(updatesystem);
                }
            }
            UpdatePageMsg::OpenRow(row, pkgtype) => match pkgtype {
                InstallType::User => {
                    let updateuserlist_guard = self.updateuserlist.guard();
                    if let Some(item) = updateuserlist_guard.get(row)
                        && let Some(pkg) = &item.item.pkg
                    {
                        let _ = sender.output(AppMsg::OpenPkg(pkg.to_string()));
                    }
                }
                InstallType::System => {
                    let updatesystemlist_guard = self.updatesystemlist.guard();
                    if let Some(item) = updatesystemlist_guard.get(row)
                        && let Some(pkg) = &item.item.pkg
                    {
                        let _ = sender.output(AppMsg::OpenPkg(pkg.to_string()));
                    }
                }
            },
            UpdatePageMsg::UpdateSystem => {
                let online = checkonline();
                if !online {
                    let _ = sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                let systype = self.systype.clone();
                let systemconfig = self.config.systemconfig.clone();
                let workersender = self.updateworker.sender().clone();
                let output = sender.output_sender().clone();
                REBUILD_BROKER.send(RebuildMsg::Show);
                relm4::spawn(async move {
                    let uninstallsys =
                        match systype {
                            SystemPkgs::Flake => nix_data_xinux::cache::flakes::unavailablepkgs(&[
                                &systemconfig.unwrap(),
                            ])
                            .await
                            .unwrap_or_default(),
                            SystemPkgs::None => HashMap::new(),
                        };
                    if uninstallsys.is_empty() {
                        let _ = workersender.send(UpdateAsyncHandlerMsg::UpdateSystem);
                    } else {
                        warn!("Uninstalling unavailable packages: {:?}", uninstallsys);
                        let _ = output.send(AppMsg::GetUnavailableItems(
                            HashMap::new(),
                            uninstallsys,
                            UpdateType::System,
                        ));
                    }
                });
            }
            UpdatePageMsg::UpdateSystemRm(pkgs) => {
                info!("UpdatePageMsg::UpdateSystemRm({:?})", pkgs);
                self.updateworker
                    .emit(UpdateAsyncHandlerMsg::UpdateSystemRemove(pkgs));
            }
            UpdatePageMsg::UpdateUser(pkg) => {
                info!("UPDATE USER PKG: {}", pkg);
                warn!("unimplemented");
            }
            UpdatePageMsg::UpdateAllUser => {
                let online = checkonline();
                if !online {
                    let _ = sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                REBUILD_BROKER.send(RebuildMsg::Show);
                let workersender = self.updateworker.sender().clone();
                let output = sender.output_sender().clone();
                relm4::spawn(async move {
                    let uninstalluser = nix_data_xinux::cache::profile::unavailablepkgs()
                        .await
                        .unwrap_or_default();
                    if uninstalluser.is_empty() {
                        let _ = workersender.send(UpdateAsyncHandlerMsg::UpdateUserPkgs);
                    } else {
                        warn!("Uninstalling unavailable packages: {:?}", uninstalluser);
                        let _ = output.send(AppMsg::GetUnavailableItems(
                            uninstalluser,
                            HashMap::new(),
                            UpdateType::User,
                        ));
                    }
                });
            }
            UpdatePageMsg::UpdateAllUserRm(pkgs) => {
                info!("UpdatePageMsg::UpdateAllUserRm({:?})", pkgs);
                self.updateworker
                    .emit(UpdateAsyncHandlerMsg::UpdateUserPkgsRemove(pkgs));
            }
            UpdatePageMsg::UpdateAll => {
                let online = checkonline();
                if !online {
                    let _ = sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                info!("UpdatePageMsg::UpdateAll");
                let systype = self.systype.clone();
                let systemconfig = self.config.systemconfig.clone();
                let workersender = self.updateworker.sender().clone();
                let output = sender.output_sender().clone();
                REBUILD_BROKER.send(RebuildMsg::Show);
                relm4::spawn(async move {
                    let uninstallsys =
                        match systype {
                            SystemPkgs::Flake => nix_data_xinux::cache::flakes::unavailablepkgs(&[
                                &systemconfig.unwrap(),
                            ])
                            .await
                            .unwrap_or_default(),
                            SystemPkgs::None => HashMap::new(),
                        };
                    let uninstalluser = nix_data_xinux::cache::profile::unavailablepkgs()
                        .await
                        .unwrap_or_default();
                    if uninstallsys.is_empty() && uninstalluser.is_empty() {
                        let _ = workersender.send(UpdateAsyncHandlerMsg::UpdateAll);
                    } else {
                        warn!(
                            "Uninstalling unavailable user packages: {:?}",
                            uninstalluser
                        );
                        warn!(
                            "Uninstalling unavailable system packages: {:?}",
                            uninstallsys
                        );
                        let _ = output.send(AppMsg::GetUnavailableItems(
                            uninstalluser,
                            uninstallsys,
                            UpdateType::All,
                        ));
                    }
                });
            }
            UpdatePageMsg::UpdateAllRm(userpkgs, syspkgs) => {
                info!("UpdatePageMsg::UpdateAllRm({:?}, {:?})", userpkgs, syspkgs);
                self.updateworker
                    .emit(UpdateAsyncHandlerMsg::UpdateAllRemove(userpkgs, syspkgs));
            }
            UpdatePageMsg::DoneWorking => {
                let _ = nix_data_xinux::utils::refreshicons();
                REBUILD_BROKER.send(RebuildMsg::FinishSuccess);
                let _ = sender.output(AppMsg::UpdateInstalledPkgs);
            }
            UpdatePageMsg::FailedWorking => {
                REBUILD_BROKER.send(RebuildMsg::FinishError(None));
            }
            UpdatePageMsg::UpdateOnline(online) => {
                self.set_online(online);
            }
            UpdatePageMsg::Noop => {}
        }
    }
}
