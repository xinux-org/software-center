use adw::gtk::prelude::GtkApplicationExt;
use adw::prelude::AdwDialogExt;
use gettextrs::gettext;
use relm4::adw;
use relm4::prelude::*;

pub struct ShortcutsDialog;

impl SimpleComponent for ShortcutsDialog {
    type Root = adw::ShortcutsDialog;
    type Widgets = adw::ShortcutsDialog;
    type Init = ();
    type Input = ();
    type Output = ();

    fn init_root() -> Self::Root {
        adw::ShortcutsDialog::builder().build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = root.clone();

        let navigation_section = adw::ShortcutsSection::new(Some(&gettext("Navigation")));

        navigation_section.add(adw::ShortcutsItem::new(
            &gettext("Open Explore Page"),
            "<Control>E",
        ));
        navigation_section.add(adw::ShortcutsItem::new(
            &gettext("Open Installed Page"),
            "<Control>D",
        ));
        navigation_section.add(adw::ShortcutsItem::new(
            &gettext("Open Search Page"),
            "<Control>F",
        ));

        let general_section = adw::ShortcutsSection::new(Some(&gettext("General")));

        general_section.add(adw::ShortcutsItem::new(
            &gettext("Open Preferences"),
            "<Control>comma",
        ));
        general_section.add(adw::ShortcutsItem::new(
            &gettext("Open Shortcuts"),
            "<Control>question",
        ));
        general_section.add(adw::ShortcutsItem::new(&gettext("Quit"), "<Control>q"));

        widgets.add(navigation_section);
        widgets.add(general_section);
        widgets.present(Some(&relm4::main_adw_application().windows()[0]));
        ComponentParts { model, widgets }
    }
}
