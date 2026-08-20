use adw::prelude::*;
use gettextrs::gettext;
use relm4::{factory::*, *};

use super::components::installed_item::{InstalledItem, InstalledItemModel, InstalledItemMsg};
use crate::ui::{
    package::package_page::{InstallType, NotifyPage, PkgAction, WorkPkg},
    window::{AppMsg, INSTALLED_PACKAGES_STATE, SystemPkgs},
};

#[tracker::track]
#[derive(Debug)]
pub struct InstalledPageModel {
    #[tracker::no_eq]
    installeduserlist: FactoryVecDeque<InstalledItemModel>,
    #[tracker::no_eq]
    installedsystemlist: FactoryVecDeque<InstalledItemModel>,
    systempkgtype: SystemPkgs,
    updatetracker: u8,
}

#[derive(Debug)]
pub enum InstalledPageMsg {
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    UpdatePkgTypes(SystemPkgs),
    OpenRow(usize, InstallType),
    Remove(InstalledItem),
    UnsetBusy(WorkPkg),
}

#[relm4::component(pub)]
impl SimpleComponent for InstalledPageModel {
    type Init = SystemPkgs;
    type Input = InstalledPageMsg;
    type Output = AppMsg;
    type Widgets = InstalledPageWidgets;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(InstalledPageModel::updatetracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            if !model.installeduserlist.is_empty() || !model.installedsystemlist.is_empty() {
                adw::Clamp {
                    set_maximum_size: 1000,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Start,
                        set_margin_vertical: 15,
                        set_spacing: 15,
                        gtk::Label {
                            #[watch]
                            set_visible: !model.installeduserlist.is_empty(),
                            set_halign: gtk::Align::Start,
                            add_css_class: "title-4",
                            set_lines: 1,
                            #[watch]
                            set_label: &format!(
                                "{} — {}",
                                gettext("User (nix profile)"),
                                model.installeduserlist.len()
                            ),
                        },
                        #[local_ref]
                        installeduserlist -> gtk::FlowBox {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_selection_mode: gtk::SelectionMode::None,
                            set_homogeneous: true,
                            set_max_children_per_line: 3,
                            set_min_children_per_line: 1,
                            set_column_spacing: 10,
                            set_row_spacing: 10,
                            connect_child_activated[sender] => move |_, child| {
                                sender.input(InstalledPageMsg::OpenRow(child.index() as usize, InstallType::User))
                            }
                        },
                        gtk::Label {
                            #[watch]
                            set_visible: !model.installedsystemlist.is_empty(),
                            set_halign: gtk::Align::Start,
                            add_css_class: "title-4",
                            set_lines: 1,
                            #[watch]
                            set_label: &format!(
                                "{} — {}",
                                gettext("System (configuration.nix)"),
                                model.installedsystemlist.len()
                            ),
                        },
                        #[local_ref]
                        installedsystemlist -> gtk::FlowBox {
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_selection_mode: gtk::SelectionMode::None,
                            set_homogeneous: true,
                            set_max_children_per_line: 3,
                            set_min_children_per_line: 1,
                            set_column_spacing: 10,
                            set_row_spacing: 10,
                            connect_child_activated[sender] => move |_, child| {
                                sender.input(InstalledPageMsg::OpenRow(child.index() as usize, InstallType::System))
                            }
                        },
                    }
                }
            } else {
                adw::StatusPage {
                    set_icon_name: Some("library-symbolic"),
                    set_title: &gettext("No apps found"),
                }
            }
        }
    }

    fn init(
        systempkgtype: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            InstalledPageMsg::UpdateInstalledPackages {
                system_packages: state.installed_system_packages.clone(),
                user_packages: state.installed_user_packages.clone(),
            }
        });

        let model = InstalledPageModel {
            installeduserlist: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(
                    sender.input_sender(),
                    |installed_item_msg| match installed_item_msg {
                        InstalledItemMsg::Delete(item) => InstalledPageMsg::Remove(item),
                    },
                ),
            installedsystemlist: FactoryVecDeque::builder()
                .launch(gtk::FlowBox::new())
                .forward(
                    sender.input_sender(),
                    |installed_item_msg| match installed_item_msg {
                        InstalledItemMsg::Delete(item) => InstalledPageMsg::Remove(item),
                    },
                ),
            updatetracker: 0,
            systempkgtype,
            tracker: 0,
        };

        let installeduserlist = model.installeduserlist.widget();
        let installedsystemlist = model.installedsystemlist.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstalledPageMsg::UpdateInstalledPackages {
                system_packages,
                user_packages,
            } => {
                self.update_updatetracker(|_| ());

                let mut installeduserlist_guard = self.installeduserlist.guard();
                installeduserlist_guard.clear();
                for item in user_packages {
                    installeduserlist_guard.push_back(item);
                }

                let mut installedsystemlist_guard = self.installedsystemlist.guard();
                installedsystemlist_guard.clear();
                for item in system_packages {
                    installedsystemlist_guard.push_back(item);
                }
            }
            InstalledPageMsg::UpdatePkgTypes(systempkgtype) => self.systempkgtype = systempkgtype,
            InstalledPageMsg::OpenRow(row, pkgtype) => match pkgtype {
                InstallType::User => {
                    let installeduserlist_guard = self.installeduserlist.guard();
                    if let Some(item) = installeduserlist_guard.get(row) {
                        let _ = sender.output(AppMsg::OpenPkg(item.item.pkg.to_string()));
                    }
                }
                InstallType::System => {
                    let installedsystemlist_guard = self.installedsystemlist.guard();
                    if let Some(item) = installedsystemlist_guard.get(row) {
                        let _ = sender.output(AppMsg::OpenPkg(item.item.pkg.to_string()));
                    }
                }
            },
            InstalledPageMsg::Remove(item) => {
                let work = WorkPkg {
                    pkg: item.pkg,
                    pname: item.pname,
                    pkgtype: item.pkgtype,
                    action: PkgAction::Remove,
                    block: false,
                    notify: Some(NotifyPage::Installed),
                };
                let _ = sender.output(AppMsg::AddInstalledToWorkQueue(work));
            }
            InstalledPageMsg::UnsetBusy(work) => match work.pkgtype {
                InstallType::User => {
                    let mut installeduserlist_guard = self.installeduserlist.guard();
                    for i in 0..installeduserlist_guard.len() {
                        if let Some(item) = installeduserlist_guard.get_mut(i)
                            && item.item.pname == work.pname
                            && item.item.pkgtype == work.pkgtype
                        {
                            item.item.busy = false;
                        }
                    }
                }
                InstallType::System => {
                    let mut installedsystemlist_guard = self.installedsystemlist.guard();
                    for i in 0..installedsystemlist_guard.len() {
                        if let Some(item) = installedsystemlist_guard.get_mut(i)
                            && item.item.pkg == work.pkg
                            && item.item.pkgtype == work.pkgtype
                        {
                            item.item.busy = false;
                        }
                    }
                }
            },
        }
    }
}
