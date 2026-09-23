use relm4::{
    FactorySender, RelmWidgetExt, adw,
    factory::{DynamicIndex, FactoryComponent},
    gtk::{self, pango::EllipsizeMode, prelude::*},
};

#[derive(Debug)]
pub struct CarouselTileModel {
    name: String,
    summary: String,
    icon: String,
    screenshot: String,
}

#[derive(Debug)]
pub enum CarouselTileInput {}

#[derive(Debug)]
pub enum CarouselTileOutput {}

pub struct CarouselTileInit {}

#[relm4::factory(pub)]
impl FactoryComponent for CarouselTileModel {
    type ParentWidget = adw::Carousel;
    type Input = CarouselTileInput;
    type Output = CarouselTileOutput;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            inline_css: "background-color: #114b91;",
            set_hexpand: true,
            gtk::Box {
                set_halign: gtk::Align::Center,
                set_hexpand: true,
                set_spacing: 128,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,
                    set_spacing: 12,
                    set_margin_vertical: 50,
                    gtk::Image {
                        #[watch]
                        set_from_file: Some(&self.icon),
                        set_pixel_size: 128,
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::Label {
                            #[watch]
                            set_label: &self.name,
                            add_css_class: "title-1",
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &self.summary,
                            set_ellipsize: EllipsizeMode::End,
                            add_css_class: "caption",
                        },
                    },
                },
                gtk::Box {
                    set_width_request: 620,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Picture {
                        set_margin_top: 40,
                        #[watch]
                        set_filename: Some(&self.screenshot),
                    },
                },
            }
        }
    }

    fn init_model(_init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        // let file = gio::File::for_path("");
        Self {
            name: "Bazaar".to_string(),
            summary: "Discover and install apps".to_string(),
            icon: "/nix/store/lqirp0agcqwmh52f3mjq6081ci2z8g68-nix-software-center-0.2.0/share/app-info/icons/nixos/128x128/bazaar_io.github.kolunmi.Bazaar.png".to_string(),
            screenshot: "/home/dior/.cache/nix-software-center/screenshots/61aceea0fb1ce01fe449f1d6bd075da021c2e0300e6fcfad40d156b293e1619d_croppped.png".to_string(),
        }
    }
}
