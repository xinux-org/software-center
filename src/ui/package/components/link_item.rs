use enum_assoc::Assoc;
use gettextrs::gettext;
use relm4::{
    FactorySender,
    adw::{self, gio, prelude::*},
    gtk,
    prelude::*,
};

#[derive(Debug)]
pub struct LinkItem {
    link_type: LinkType,
    link: String,
}

#[derive(Debug, Assoc)]
#[func(pub fn title(&self) -> String)]
#[func(pub const fn icon(&self) -> Option<&'static str>)]
pub enum LinkType {
    #[assoc(title = gettext("Project Website"), icon = "globe-symbolic")]
    Website,
    #[assoc(title = gettext("Issue Tracker"), icon = "sad-computer-symbolic")]
    IssueTracker,
    #[assoc(title = gettext("FAQ"), icon = "question-round-outline-symbolic")]
    FAQ,
    #[assoc(title = gettext("Help"), icon = "rescue-symbolic")]
    Help,
    #[assoc(title = gettext("Donate"), icon = "heart-filled-symbolic")]
    Donate,
    #[assoc(title = gettext("Translate"), icon = "keyboard-layout-symbolic")]
    Translate,
    #[assoc(title = gettext("Contact"), icon = "mail-send-symbolic")]
    Contact,
    #[assoc(title = gettext("Source Code"), icon = "code-symbolic")]
    Source,
    #[assoc(title = gettext("Contribute"), icon = "people-symbolic")]
    Contribute,
    #[assoc(title = gettext("Nix Source"), icon = "shoe-box-symbolic")]
    NixSource,
}

#[derive(Debug)]
pub struct LinkItemInit {
    pub link_type: LinkType,
    pub link: String,
}

#[derive(Debug)]
pub enum LinkItemMsg {
    ShowToast(String),
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
            set_title: &self.link_type.title(),
            set_subtitle: &self.link,
            add_prefix = &gtk::Image {
                set_icon_name: self.link_type.icon(),
            },
            add_suffix = &gtk::Box {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,
                set_spacing: 4,
                gtk::Button {
                    add_css_class: "flat",
                    set_icon_name: "copy-symbolic",
                    connect_clicked[sender, link = self.link.to_string()] => move |btn| {
                        btn.clipboard().set_text(&link);
                        sender.output(LinkItemMsg::ShowToast(gettext("Copied!")));
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
            connect_activated[link = self.link.clone()] => move |_| {
                gio::AppInfo::launch_default_for_uri(&link, gio::AppLaunchContext::NONE);
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
