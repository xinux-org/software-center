use std::collections::HashMap;

use rand::seq::SliceRandom;
use relm4::{
    Component, ComponentParts, ComponentSender, adw,
    factory::FactoryVecDeque,
    gtk::{self, prelude::*},
};

use crate::ui::{
    explore::components::carousel_tile::{CarouselTileInit, CarouselTileModel},
    package::components::package_tile::PkgTile,
    windowloading::APPSTREAM_DATA_STATE,
};

#[derive(Debug)]
pub struct CarouselModel {
    tiles: FactoryVecDeque<CarouselTileModel>,

    active_page: u32,
}

#[derive(Debug)]
pub enum CarouselInput {
    SetPackages(Vec<PkgTile>),
    PageChanged(u32),
    PreviousPage,
    NextPage,
}

#[relm4::component(pub)]
impl Component for CarouselModel {
    type CommandOutput = ();
    type Input = CarouselInput;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        gtk::Box {
            add_css_class: "rounded",
            set_orientation: gtk::Orientation::Vertical,
            set_valign: gtk::Align::Start,
            set_overflow: gtk::Overflow::Hidden,
            #[watch]
            set_visible: !model.tiles.is_empty(),
            gtk::Overlay {
                set_valign: gtk::Align::Start,
                #[local_ref]
                tiles_factory -> adw::Carousel {
                    connect_page_changed[sender] => move |_carousel, page| {
                        sender.input(CarouselInput::PageChanged(page));
                    },
                },
                add_overlay = &gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                    #[watch]
                    set_reveal_child: !model.tiles.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Fill,
                    gtk::Button {
                        set_can_focus: false,
                        set_width_request: 60,
                        add_css_class: "flat",
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        set_icon_name: "go-previous-symbolic",
                        connect_clicked[sender] => move |_| {
                            sender.input(CarouselInput::PreviousPage);
                        },
                    },
                },
                add_overlay = &gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                    #[watch]
                    set_reveal_child: !model.tiles.is_empty(),
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Fill,
                    gtk::Button {
                        set_can_focus: false,
                        set_width_request: 60,
                        add_css_class: "flat",
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        set_icon_name: "go-next-symbolic",
                        connect_clicked[sender] => move |_| {
                            sender.input(CarouselInput::NextPage);
                        },
                    },
                },
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tiles = FactoryVecDeque::builder()
            .launch(adw::Carousel::new())
            .detach();

        let model = Self {
            tiles,
            active_page: 0,
        };

        let tiles_factory = model.tiles.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CarouselInput::SetPackages(package_tiles) => {
                let appstream_data = APPSTREAM_DATA_STATE.read();

                let mut screenshots = HashMap::new();

                for package_tile in package_tiles {
                    if let Some(app_data) = appstream_data.get(&package_tile.pkg)
                        && app_data.icon.is_some()
                        && let Some(app_screenshots) = app_data.screenshots.as_ref()
                        && let Some(screenshot) = app_screenshots
                            .iter()
                            .find(|screenshot| screenshot.default.unwrap_or_default())
                            .or_else(|| app_screenshots.first())
                        && let Some(source_image) = screenshot.sourceimage.as_ref()
                    {
                        screenshots.insert(
                            package_tile.pkg.clone(),
                            (package_tile, source_image.url.clone()),
                        );
                    }
                }

                let mut featured = screenshots.into_iter().collect::<Vec<_>>();
                let mut rng = rand::rng();
                featured.shuffle(&mut rng);
                let featured = featured.into_iter().take(5).collect::<Vec<_>>();

                let carousel_tiles =
                    featured
                        .into_iter()
                        .map(
                            |(package, (package_tile, screenshot_url))| CarouselTileInit {
                                package,
                                name: package_tile.name,
                                summary: package_tile.summary,
                                icon: package_tile.icon.unwrap_or_default(),
                                screenshot: screenshot_url,
                            },
                        );

                let mut guard = self.tiles.guard();
                for tile in carousel_tiles {
                    guard.push_back(tile);
                }
            }
            CarouselInput::PageChanged(page) => {
                self.active_page = page;
            }
            CarouselInput::PreviousPage => {
                let pages = self.tiles.len() as u32;
                let carousel = self.tiles.widget();

                if self.active_page == 0 && pages >= 1 {
                    let widget = carousel.nth_page(pages - 1);
                    carousel.scroll_to(&widget, true);
                } else {
                    let widget = carousel.nth_page(self.active_page - 1);
                    carousel.scroll_to(&widget, true);
                }
            }
            CarouselInput::NextPage => {
                let pages = self.tiles.len() as u32;
                let carousel = self.tiles.widget();

                if self.active_page >= pages - 1 {
                    let widget = carousel.nth_page(0);
                    carousel.scroll_to(&widget, true);
                } else {
                    let widget = carousel.nth_page(self.active_page + 1);
                    carousel.scroll_to(&widget, true);
                }
            }
        }
    }
}
