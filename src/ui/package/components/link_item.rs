use gettextrs::gettext;
use relm4::adw::prelude::*;
use relm4::{factory::*, *};

#[derive(Debug)]
pub struct LinkItem {
    link_type: LinkType,
    link: String,
}

#[derive(Debug)]
pub enum LinkType {
    Website,
    IssueTracker,
    FAQ,
    Help,
    Donate,
    Translate,
    Contact,
    Source,
    Contribute,
}

#[derive(Debug)]
pub struct LinkItemInit {
    pub link_type: LinkType,
    pub link: String,
}

#[derive(Debug)]
pub enum LinkItemMsg {
    Open,
    Copy,
}

#[relm4::factory(pub)]
impl FactoryComponent for LinkItem {
    type CommandOutput = ();
    type Init = LinkItemInit;
    type Input = ();
    type Output = LinkItemMsg;
    type ParentWidget = adw::PreferencesGroup;

    view! {
        adw::ActionRow {
            set_activatable: true,
            set_title: &match self.link_type {
                LinkType::Website => gettext("Project Website"),
                LinkType::IssueTracker => gettext("Issue Tracker"),
                LinkType::FAQ => gettext("FAQ"),
                LinkType::Help => gettext("Help"),
                LinkType::Donate => gettext("Donate"),
                LinkType::Translate => gettext("Translate"),
                LinkType::Contact => gettext("Contact"),
                LinkType::Source => gettext("Source Code"),
                LinkType::Contribute => gettext("Contribute"),
            },
            set_subtitle: &self.link,
            add_prefix = &gtk::Image {
                set_icon_name: match self.link_type {
                    LinkType::Website => Some("globe-symbolic"),
                    LinkType::IssueTracker => Some("sad-computer-symbolic"),
                    LinkType::FAQ => Some("question-round-outline-symbolic"),
                    LinkType::Help => Some("rescue-symbolic"),
                    LinkType::Donate => Some("heart-filled-symbolic"),
                    LinkType::Translate => Some("keyboard-layout-symbolic"),
                    LinkType::Contact => Some("mail-send-symbolic"),
                    LinkType::Source => Some("code-symbolic"),
                    LinkType::Contribute => Some("people-symbolic"),
                },
            },
            add_suffix = &gtk::Box {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,
                set_spacing: 4,
                gtk::Button {
                    add_css_class: "flat",
                    set_icon_name: "copy-symbolic",
                    connect_clicked[link = self.link.to_string()] => move |btn| {
                        btn.clipboard().set_text(&link);
                    },
                },
                gtk::Separator {
                    set_margin_top: 6,
                    set_margin_bottom: 6,
                },
                gtk::Image {
                    set_margin_start: 8,
                    set_margin_end: 4,
                    set_icon_name: Some("external-link-symbolic"),
                },
            },
        },
    }

    fn init_model(parent: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            link: parent.link,
            link_type: parent.link_type,
        }
    }
}
