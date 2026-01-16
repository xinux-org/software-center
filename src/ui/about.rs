use adw::prelude::*;
use gettextrs::gettext;
use relm4::*;

use crate::config;

#[derive(Debug)]
pub struct AboutPageModel {}

#[derive(Debug)]
pub enum AboutPageMsg {
    Show,
    Hide,
}

pub struct Widgets {
    parent_window: gtk::Window,
}

impl SimpleComponent for AboutPageModel {
    type Init = gtk::Window;
    type Widgets = Widgets;
    type Input = ();
    type Output = ();
    type Root = ();

    fn init_root() -> Self::Root {}

    fn init(
        parent_window: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};

        let widgets = Widgets { parent_window };

        ComponentParts { model, widgets }
    }

    fn update_view(&self, _dialog: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let dialog = adw::AboutDialog::builder()
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
            .build();
        dialog.present(relm4::main_application().active_window().as_ref());
    }
}
