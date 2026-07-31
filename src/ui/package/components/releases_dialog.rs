use adw::prelude::*;
use gettextrs::gettext;
use log::*;
use relm4::{factory::FactoryVecDeque, *};

use super::release_item::{ReleaseItem, ReleaseItemInit};

#[tracker::track]
#[derive(Debug)]
pub struct ReleasesDialog {
    #[tracker::no_eq]
    releases: FactoryVecDeque<ReleaseItem>,
}

#[derive(Debug)]
pub enum ReleasesMsg {
    Show(Vec<ReleaseItemInit>),
    Ignore,
}

#[relm4::component(pub)]
impl Component for ReleasesDialog {
    type Init = ();
    type Input = ReleasesMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::PreferencesDialog {
            set_vexpand: true,
            set_search_enabled: false,
            set_title: &gettext("Version History"),
            add = &adw::PreferencesPage {
                #[local_ref]
                releases_factory -> adw::PreferencesGroup {},
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ReleasesDialog {
            releases: FactoryVecDeque::builder()
                .launch(adw::PreferencesGroup::new())
                .detach(),
            tracker: 0,
        };

        let releases_factory = model.releases.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        self.reset();
        match msg {
            ReleasesMsg::Show(releases) => {
                let mut releases_factory = self.releases.guard();
                releases_factory.clear();
                for release in releases {
                    releases_factory.push_back(release.clone());
                }

                let window = relm4::main_application().active_window();
                root.present(window.as_ref());
            }
            _ => {}
        }
    }
}
