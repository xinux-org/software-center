use std::convert::identity;

use gettextrs::gettext;
use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent,
    adw::{self, prelude::*},
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    factory::FactoryVecDeque,
    gtk,
};

use crate::ui::{
    package::package_page::{
        InstallType, NotifyPage, PackageAction, PackagePageInit, PackagePageModel, WorkPackage,
    },
    window::{AppMsg, INSTALLED_PACKAGES_STATE, SystemPkgs},
};

use super::components::installed_item::{InstalledItem, InstalledItemModel, InstalledItemMsg};

#[tracker::track]
#[derive(Debug)]
pub struct InstalledPageModel {
    navigation: adw::NavigationView,

    system_package_type: SystemPkgs,

    #[tracker::no_eq]
    installeduserlist: FactoryVecDeque<InstalledItemModel>,
    #[tracker::no_eq]
    installedsystemlist: FactoryVecDeque<InstalledItemModel>,

    #[tracker::no_eq]
    package_page: Option<AsyncController<PackagePageModel>>,
}

#[derive(Debug)]
pub enum InstalledPageMsg {
    UpdateInstalledPackages {
        system_packages: Vec<InstalledItem>,
        user_packages: Vec<InstalledItem>,
    },
    UpdatePkgTypes(SystemPkgs),
    OpenRow(usize, InstallType),
    OpenPackage(String),
    Remove(InstalledItem),
    UnsetBusy(WorkPackage),
}

#[relm4::component(pub)]
impl SimpleComponent for InstalledPageModel {
    type Init = SystemPkgs;
    type Input = InstalledPageMsg;
    type Output = AppMsg;
    type Widgets = InstalledPageWidgets;

    view! {
        #[name = "navigation"]
        adw::NavigationView {
            add = &adw::NavigationPage {
                adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {},
                    gtk::ScrolledWindow {
                        set_hscrollbar_policy: gtk::PolicyType::Never,
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
                },
            },
        },
    }

    fn init(
        system_package_type: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        INSTALLED_PACKAGES_STATE.subscribe(sender.input_sender(), |state| {
            InstalledPageMsg::UpdateInstalledPackages {
                system_packages: state.installed_system_packages.clone(),
                user_packages: state.installed_user_packages.clone(),
            }
        });

        let mut model = InstalledPageModel {
            navigation: adw::NavigationView::new(),
            system_package_type,
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
            package_page: None,
            tracker: 0,
        };

        let installeduserlist = model.installeduserlist.widget();
        let installedsystemlist = model.installedsystemlist.widget();

        let widgets = view_output!();

        model.navigation = widgets.navigation.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstalledPageMsg::UpdateInstalledPackages {
                system_packages,
                user_packages,
            } => {
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
            InstalledPageMsg::UpdatePkgTypes(system_package_type) => {
                self.set_system_package_type(system_package_type)
            }
            InstalledPageMsg::OpenRow(row, pkgtype) => match pkgtype {
                InstallType::User => {
                    let installeduserlist_guard = self.installeduserlist.guard();
                    if let Some(item) = installeduserlist_guard.get(row) {
                        let _ =
                            sender.input(InstalledPageMsg::OpenPackage(item.item.pkg.to_string()));
                    }
                }
                InstallType::System => {
                    let installedsystemlist_guard = self.installedsystemlist.guard();
                    if let Some(item) = installedsystemlist_guard.get(row) {
                        let _ =
                            sender.input(InstalledPageMsg::OpenPackage(item.item.pkg.to_string()));
                    }
                }
            },
            InstalledPageMsg::OpenPackage(package) => {
                let package_page = PackagePageModel::builder()
                    .launch(PackagePageInit {
                        package,
                        syspkgs: self.system_package_type.clone(),
                    })
                    .forward(sender.output_sender(), identity);
                self.navigation.push(package_page.widget());
                self.set_package_page(Some(package_page));
            }
            InstalledPageMsg::Remove(item) => {
                let work = WorkPackage {
                    package: item.pkg,
                    package_name: item.pname,
                    install_type: item.pkgtype,
                    action: PackageAction::Remove,
                    block: false,
                    notify: Some(NotifyPage::Installed),
                };
                let _ = sender.output(AppMsg::AddInstalledToWorkQueue(work));
            }
            InstalledPageMsg::UnsetBusy(work) => match work.install_type {
                InstallType::User => {
                    let mut installeduserlist_guard = self.installeduserlist.guard();
                    for i in 0..installeduserlist_guard.len() {
                        if let Some(item) = installeduserlist_guard.get_mut(i)
                            && item.item.pname == work.package_name
                            && item.item.pkgtype == work.install_type
                        {
                            item.item.busy = false;
                        }
                    }
                }
                InstallType::System => {
                    let mut installedsystemlist_guard = self.installedsystemlist.guard();
                    for i in 0..installedsystemlist_guard.len() {
                        if let Some(item) = installedsystemlist_guard.get_mut(i)
                            && item.item.pkg == work.package
                            && item.item.pkgtype == work.install_type
                        {
                            item.item.busy = false;
                        }
                    }
                }
            },
        }
    }
}
