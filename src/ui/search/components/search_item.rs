use log::*;
use relm4::{
    adw::{self, prelude::*},
    factory::*,
    gtk::pango,
    *,
};
use std::path::Path;

use crate::APPINFO;

#[derive(Default, Debug, PartialEq, Eq)]
pub struct SearchItem {
    pub name: String,
    pub pkg: String,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub installeduser: bool,
    pub installedsystem: bool,
}

#[tracker::track]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct SearchItemModel {
    pub item: SearchItem,
}

#[derive(Debug)]
pub enum SearchItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for SearchItemModel {
    type CommandOutput = ();
    type Init = SearchItem;
    type Input = ();
    type Output = SearchItemMsg;
    type ParentWidget = adw::gtk::ListBox;
    // type ParentInput = SearchPageMsg;

    view! {
        adw::PreferencesRow {
            set_activatable: !self.item.pkg.is_empty(),
            set_can_focus: false,
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
                gtk::Overlay {
                    add_overlay = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::End,
                        gtk::Image {
                            add_css_class: "accent",
                            set_valign: gtk::Align::Start,
                            set_halign: gtk::Align::End,
                            set_icon_name: Some("emblem-default-symbolic"),
                            #[watch]
                            set_visible: self.item.installeduser,
                        },
                        gtk::Image {
                            add_css_class: "success",
                            set_valign: gtk::Align::Start,
                            set_halign: gtk::Align::End,
                            set_icon_name: Some("emblem-default-symbolic"),
                            #[watch]
                            set_visible: self.item.installedsystem,
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
                            set_label: self.item.pkg.as_str(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: self.item.summary.as_deref().unwrap_or(""),
                            set_visible: self.item.summary.is_some(),
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_wrap: true,
                            set_max_width_chars: 0,
                        },
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

        let item = SearchItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            installeduser: parent.installeduser,
            installedsystem: parent.installedsystem,
        };

        Self { item, tracker: 0 }
    }
}
