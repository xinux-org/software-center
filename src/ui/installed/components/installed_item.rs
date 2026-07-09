use adw::prelude::*;
use relm4::{factory::*, gtk::pango, *};
use std::path::Path;

use crate::{APPINFO, ui::package::package_page::InstallType};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InstalledItem {
    pub name: String,
    pub pkg: Option<String>,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub pkgtype: InstallType,
    pub busy: bool,
    pub version: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InstalledItemModel {
    pub item: InstalledItem,
}

#[derive(Debug)]
pub enum InstalledItemMsg {
    Delete(InstalledItem),
}

#[derive(Debug)]
pub enum InstalledItemInputMsg {
    Busy(bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for InstalledItemModel {
    type CommandOutput = ();
    type Init = InstalledItem;
    type Input = InstalledItemInputMsg;
    type Output = InstalledItemMsg;
    type ParentWidget = adw::gtk::FlowBox;

    view! {
    gtk::FlowBoxChild {
        set_width_request: 270,
        adw::PreferencesRow {
            set_activatable: self.item.pkg.is_some(),
            set_can_focus: false,
            add_css_class: "card",
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_spacing: 10,
                set_margin_all: 10,
                adw::Bin {
                    set_valign: gtk::Align::Center,
                    #[wrap(Some)]
                    set_child = if self.item.icon.is_some() {
                        gtk::Image {
                            add_css_class: "icon-dropshadow",
                            set_halign: gtk::Align::Start,
                            set_from_file: {
                                if let Some(i) = &self.item.icon {
                                    let iconpath = format!("{}/icons/nixos/128x128/{}", APPINFO, i);
                                    let iconpath64 = format!("{}/icons/nixos/64x64/{}", APPINFO, i);
                                    if Path::new(&iconpath).is_file() {
                                        Some(iconpath)
                                    } else if Path::new(&iconpath64).is_file() {
                                        Some(iconpath64)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            },
                            set_pixel_size: 64,
                        }
                    } else {
                        gtk::Image {
                            add_css_class: "icon-dropshadow",
                            set_halign: gtk::Align::Start,
                            set_icon_name: Some("package-x-generic"),
                            set_pixel_size: 64,
                        }
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_spacing: 2,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: self.item.name.as_str(),
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                        set_label: &self.item.version,
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    },
                },
                if self.item.busy {
                    gtk::Spinner {
                        set_spinning: true,
                    }
                } else {
                    gtk::Button {
                        add_css_class: "destructive-action",
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::End,
                        set_icon_name: "user-trash-symbolic",
                        set_can_focus: false,
                        connect_clicked[sender, item = self.item.clone()] => move |_| {
                            sender.input(InstalledItemInputMsg::Busy(true));
                            let _ = sender.output(InstalledItemMsg::Delete(item.clone()));
                        }
                    }
                }
            }
        }
    }
    }

    fn init_model(parent: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let sum = if let Some(s) = parent.summary {
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

        let item = InstalledItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            pkgtype: parent.pkgtype,
            busy: parent.busy,
            version: parent.version,
        };

        Self { item }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            InstalledItemInputMsg::Busy(b) => self.item.busy = b,
        }
    }
}
