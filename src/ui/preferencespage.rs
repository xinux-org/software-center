use super::window::AppMsg;
use gettextrs::gettext;
use nix_data_xinux::config::configfile::NixDataConfig;
use relm4::{
    adw::{self, prelude::*},
    gtk::{self, glib},
    *,
};
use relm4_components::open_dialog::*;
use std::path::{Path, PathBuf};

#[tracker::track]
#[derive(Debug)]
pub struct PreferencesPageModel {
    configpath: Option<PathBuf>,
    flake: Option<PathBuf>,
    flakearg: Option<String>,
    flake_pending: bool,
    #[tracker::no_eq]
    open_dialog: Controller<OpenDialog>,
    #[tracker::no_eq]
    flake_file_dialog: Controller<OpenDialog>,
}

#[derive(Debug)]
pub enum PreferencesPageMsg {
    Show(NixDataConfig),
    Open,
    OpenFlake,
    SetConfigPath(Option<PathBuf>),
    SetFlakePath(Option<PathBuf>),
    SetFlakeArg(Option<String>),
    ModifyFlake,
    CancelFlakeSelection,
    Ignore,
}

#[relm4::component(pub)]
impl Component for PreferencesPageModel {
    type Init = ();
    type Input = PreferencesPageMsg;
    type Output = AppMsg;
    // type Widgets = PreferencesPageWidgets;
    type CommandOutput = ();

    view! {
        adw::PreferencesDialog {
            set_vexpand: true,
            set_search_enabled: false,
            add = &adw::PreferencesPage {
                add = &adw::PreferencesGroup {
                    // set_title: "Preferences",
                    set_visible: Path::new("/etc/nixos").exists(),
                    add = &adw::ActionRow {
                        set_title: &gettext("Configuration file"),
                        add_suffix = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_spacing: 10,
                            gtk::Button {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 5,
                                    gtk::Image {
                                        set_icon_name: Some("document-open-symbolic"),
                                    },
                                    gtk::Label {
                                        #[watch]
                                        set_label: &model // T1
                                            .configpath // Option<T2>
                                            .as_ref() // Option<&T2>
                                            // Option<&T2> -|.and_then|-> (&T2 -> Option<T5>) -> Option<T5>
                                            .and_then(|cp| { // cp = &T2
                                                cp.file_name() // Option<&T3>
                                                    .and_then(|st| st.to_str()) // Option<&T3> -|.and_then|-> (&T3 -> Option<&T4>) -> Option<&T4>
                                                    .map(|st| st.to_string()) // Option<&T4> -> (&T4 -> T5) -> Option<T5>
                                            })
                                            // Option<T5> -|.unwrap_or(T5)|-> T5
                                            .unwrap_or(gettext("(None)"))
                                    }
                                },
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::Open);
                                }
                            },
                            gtk::Button {
                                set_icon_name: "user-trash-symbolic",
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::SetConfigPath(None));
                                }
                            }
                        }
                    },
                    add = &adw::ActionRow {
                        set_title: &gettext("Use nix flakes"),
                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            connect_state_set[sender] => move |_, b| {
                                if b {
                                    sender.input(PreferencesPageMsg::OpenFlake);
                                    glib::Propagation::Stop
                                } else {
                                    sender.input(PreferencesPageMsg::SetFlakePath(None));
                                    glib::Propagation::Proceed
                                }
                            } @switched,
                            #[track(model.changed(PreferencesPageModel::flake()))]
                            #[block_signal(switched)]
                            set_state: model.flake.is_some()
                        }
                    },
                    add = &adw::ActionRow {
                        set_title: &gettext("Flake file"),
                        #[watch]
                        set_visible: model.flake.is_some(),
                        add_suffix = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_spacing: 10,
                            gtk::Button {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 5,
                                    gtk::Image {
                                        set_icon_name: Some("document-open-symbolic"),
                                    },
                                    gtk::Label {
                                        #[watch]
                                        set_label: model.flake
                                          .as_ref()
                                          .and_then(
                                            |path| path.file_name()
                                              .and_then(|file_name| file_name.to_str())
                                          ).unwrap_or(&gettext("(None)"))
                                    }
                                },
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::OpenFlake);
                                }
                            },
                            gtk::Button {
                                // add_css_class: "flat",
                                set_icon_name: "user-trash-symbolic",
                                connect_clicked[sender] => move |_| {
                                    sender.input(PreferencesPageMsg::SetFlakePath(Some(PathBuf::new())));
                                }
                            }
                        }
                    },
                    add = &adw::EntryRow {
                        #[watch]
                        set_visible: model.flake.is_some(),
                        set_title: &gettext("Flake arguments (--flake path/to/flake.nix#ENTRY)"),
                        set_use_markup: false,
                        connect_changed[sender] => move |x| {
                            sender.input(PreferencesPageMsg::SetFlakeArg({
                                let text = x.text().to_string();
                                if text.is_empty() {
                                    None
                                } else {
                                    Some(text)
                                }}));
                        } @flakeentry,
                        #[track(model.changed(PreferencesPageModel::flake()))]
                        #[block_signal(flakeentry)]
                        set_text: model.flakearg.as_ref().unwrap_or(&String::new())
                    }

                }
            }
        }
    }

    fn init(
        _parent_window: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let open_dialog = OpenDialog::builder()
            .launch(OpenDialogSettings::default())
            .forward(sender.input_sender(), |response| match response {
                OpenDialogResponse::Accept(path) => PreferencesPageMsg::SetConfigPath(Some(path)),
                OpenDialogResponse::Cancel => PreferencesPageMsg::Ignore,
            });
        let flake_file_dialog = OpenDialog::builder()
            .launch(OpenDialogSettings::default())
            .forward(sender.input_sender(), |response| match response {
                OpenDialogResponse::Accept(path) => PreferencesPageMsg::SetFlakePath(Some(path)),
                OpenDialogResponse::Cancel => PreferencesPageMsg::CancelFlakeSelection,
            });
        let model = PreferencesPageModel {
            configpath: None,
            flake: None,
            flakearg: None,
            flake_pending: false,
            open_dialog,
            flake_file_dialog,
            tracker: 0,
        };

        let widgets = view_output!();
        // widgets.present(Some(&relm4::main_application().windows()[0]));

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: Self::Input, 
        sender: ComponentSender<Self>, 
        root: &Self::Root
    ) {
        self.reset();
        match msg {
            PreferencesPageMsg::Show(config) => {
                self.configpath = config.systemconfig.as_ref().map(PathBuf::from);
                self.set_flake(config.flake.as_ref().map(PathBuf::from));
                self.set_flakearg(config.flakearg);
                
                let window = relm4::main_application().active_window();
                root.present(window.as_ref());
            }
            PreferencesPageMsg::Open => self.open_dialog.emit(OpenDialogMsg::Open),
            PreferencesPageMsg::OpenFlake => {
                self.set_flake_pending(true);
                self.flake_file_dialog.emit(OpenDialogMsg::Open);
            }
            PreferencesPageMsg::SetConfigPath(path) => {
                self.configpath = path.clone();
                let path_str = path.as_ref().map(|x| x.to_string_lossy().to_string());
                log::info!("Setting system config path: {:?}", path_str);
                
                let _ = sender.output(AppMsg::UpdateSysconfig(path_str));
                // fetch installed pkg and update after done selection
                let _ = sender.output(AppMsg::UpdateInstalledPkgs);
            }
            PreferencesPageMsg::SetFlakePath(path) => {
                log::info!("Setting flake path: {:?}", path);
                self.flake = path.clone();
                self.set_flake_pending(false);
                if path.is_none() {
                    self.set_flakearg(None);
                }
                sender.input(PreferencesPageMsg::ModifyFlake)
            }
            PreferencesPageMsg::SetFlakeArg(arg) => {
                self.flakearg = arg;
                sender.input(PreferencesPageMsg::ModifyFlake)
            }
            PreferencesPageMsg::ModifyFlake => {
                let flake_path = self.flake.as_ref().and_then(|p| {
                    if p.as_os_str().is_empty() {
                        None
                    } else {
                        Some(p.to_string_lossy().to_string())
                    }
                });
                log::info!("Updating flake config: flake={:?}, arg={:?}", flake_path, self.flakearg);
                let _ = sender.output(AppMsg::UpdateFlake(
                    flake_path,
                    self.flakearg.clone(),
                ));
            }
            PreferencesPageMsg::CancelFlakeSelection => {
                log::info!("Flake selection cancelled, reverting to previous state");
                self.set_flake_pending(false);
            }
            _ => {}
        }
    }
}
