use adw::gio;
use gettextrs::{LocaleCategory, gettext};
use gtk::{glib, prelude::ApplicationExt};
use log::{error, info};
use nix_software_center::{config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE}, ui::window::AppModel};
use relm4::*;
fn main() {
    gtk::init().unwrap();
    pretty_env_logger::init();
    glib::set_application_name(&gettext("Software Center"));

    setup_gettext();
    
    if let Ok(res) = gio::Resource::load(RESOURCES_FILE) {
        info!("Resource loaded: {}", RESOURCES_FILE);
        gio::resources_register(&res);
    } else {
        error!("Failed to load resources");
    }
    gtk::Window::set_default_icon_name(APP_ID);
    let app = adw::Application::new(
        Some(APP_ID),
        gio::ApplicationFlags::empty(),
    );
    app.set_resource_base_path(Some("/org/xinux/NixSoftwareCenter"));
    let app = RelmApp::from_app(app);
    app.run_async::<AppModel>(());
}

fn setup_gettext() {
    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to bind the text domain codeset to UTF-8");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");
}