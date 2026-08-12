use relm4::{
    adw::{self, prelude::*},
    factory::FactoryVecDeque,
    prelude::*,
};

use gettextrs::gettext;

use super::release_item::{ReleaseItem, ReleaseItemInit};

#[derive(Debug)]
pub struct ReleasesDialog {
    releases: FactoryVecDeque<ReleaseItem>,
}

#[derive(Debug)]
pub struct ReleasesInit {
    pub releases: Vec<ReleaseItemInit>,
}

#[relm4::component(pub)]
impl Component for ReleasesDialog {
    type Init = ReleasesInit;
    type Input = ();
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
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let releases = FactoryVecDeque::from_iter(init.releases, adw::PreferencesGroup::new());

        let model = ReleasesDialog { releases: releases };

        let releases_factory = model.releases.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
