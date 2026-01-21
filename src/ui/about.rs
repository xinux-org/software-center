use gettextrs::gettext;
use relm4::{
    ComponentParts, ComponentSender, SimpleComponent,
    adw::{self, prelude::*},
    gtk,
};

use crate::config;

#[derive(Debug)]
pub struct AboutPageModel {}

#[derive(Debug)]
pub enum AboutPageMsg {
    Show,
    Hide,
}

impl SimpleComponent for AboutPageModel {
    type Init = ();
    type Widgets = adw::AboutDialog;
    type Input = ();
    type Output = ();
    type Root = adw::AboutDialog;

    fn init_root() -> Self::Root {
        adw::AboutDialog::builder()
            .application_icon(config::APP_ID)
            .application_name(gettext("Nix Software Center"))
            .developer_name(gettext("Xinux Developers"))
            .developers(vec![
                "Orzklv https://github.com/orzklv",
                "Victor Fuentes https://github.com/vlinkz",
            ])
            .issue_url("https://github.com/xinux-org/software-center/issues")
            .license_type(gtk::License::Gpl30)
            .version(config::VERSION)
            .website("https://github.com/xinux-org/software-center")
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};

        let widgets = root.clone();
        widgets.present(Some(&relm4::main_application().windows()[0]));

        ComponentParts { model, widgets }
    }

    fn update_view(&self, _dialog: &mut Self::Widgets, _sender: ComponentSender<Self>) {}
}
