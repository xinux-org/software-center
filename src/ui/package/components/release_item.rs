use chrono::{DateTime, Utc};
use gettextrs::{gettext, ngettext};
use relm4::{
    adw::{self, prelude::*},
    gtk,
    prelude::*,
};

#[derive(Debug)]
pub struct ReleaseItem {
    pub version: Option<String>,
    pub date: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseItemInit {
    pub version: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub installed: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for ReleaseItem {
    type CommandOutput = ();
    type Init = ReleaseItemInit;
    type Input = ();
    type Output = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[name = "action_row"]
        adw::ActionRow {
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 15,
                set_spacing: 12,
                gtk::Box {
                    set_spacing: 8,
                    gtk::Label {
                        add_css_class: "accent",
                        add_css_class: "heading",
                        #[watch]
                        set_visible: self.version.is_some(),
                        #[watch]
                        set_label: &self.version.as_ref().map(|version| format!("{} {}", gettext("Version"), version)).unwrap_or_default(),
                    },
                    gtk::Label {
                        add_css_class: "accent",
                        add_css_class: "badge",
                        #[watch]
                        set_visible: self.installed,
                        #[watch]
                        set_label: &gettext("Installed"),
                    },
                    gtk::Label {
                        set_hexpand: true,
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Start,
                        add_css_class: "dimmed",
                        #[watch]
                        set_visible: !self.date.is_none(),
                        #[watch]
                        set_label: self.date.as_deref().unwrap_or_default(),
                    },
                },
                gtk::Label {
                    set_valign: gtk::Align::Start,
                    set_halign: gtk::Align::Start,
                    set_wrap: true,
                    set_selectable: true,
                    #[watch]
                    set_css_classes: if self.description.as_ref().map(|d| !d.is_empty()).unwrap_or(false) {&["body"]} else {&["body", "dimmed"]},
                    #[watch]
                    set_markup: &self.description.as_ref().map(|d| d.to_string()).unwrap_or_else(|| gettext("No details for this release")),
                },
                gtk::Box {
                    set_spacing: 4,
                    set_visible: self.url.is_some(),
                    gtk::Label {
                        add_css_class: "accent",
                        set_label: &format!(r#"<a href="{0}" title="{0}">{1}</a>"#, self.url.as_deref().unwrap_or_default(), gettext("Get More Information")),
                        set_tooltip: self.url.as_deref().unwrap_or_default(),
                        set_use_markup: true,
                    },
                    gtk::Image {
                        add_css_class: "accent",
                        set_icon_name: Some("external-link-symbolic"),
                        set_pixel_size: 12,
                    },
                },
            },
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let now = Utc::now();
        let date = init.date.map(|date| {
            let delta = now.signed_duration_since(date);

            let days = delta.num_days();

            match days {
                ..0 => "".to_string(),
                0 => gettext("Today"),
                1..30 => ngettext("A day ago", "%d days ago", days as u32)
                    .replace("%d", &days.to_string()),
                30..365 => ngettext("A month ago", "%d months ago", days as u32 / 30)
                    .replace("%d", &(days / 30).to_string()),
                365.. => ngettext("A year ago", "%d years ago", days as u32 / 365)
                    .replace("%d", &(days / 365).to_string()),
            }
        });

        Self {
            date: date,
            version: init.version,
            description: init.description,
            url: init.url,
            installed: init.installed,
        }
    }
}
