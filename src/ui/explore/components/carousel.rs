use std::collections::HashMap;

use rand::seq::SliceRandom;
use relm4::{
    Component, ComponentParts, ComponentSender, adw,
    factory::FactoryVecDeque,
    gtk::{self, prelude::*},
};

use crate::{
    ui::{
        explore::components::carousel_tile::{CarouselTileInit, CarouselTileModel},
        windowloading::APPSTREAM_DATA_STATE,
    },
    utils::packages::{AppData, BrandingColorScheme},
};

#[derive(Debug)]
pub struct CarouselModel {
    tiles: FactoryVecDeque<CarouselTileModel>,

    active_page: u32,
}

#[derive(Debug)]
pub enum CarouselInput {
    UpdateTiles(Vec<CarouselTileInit>),
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
        APPSTREAM_DATA_STATE.subscribe(sender.input_sender(), move |state| {
            let tiles = get_random_tiles(state, 5);
            CarouselInput::UpdateTiles(tiles)
        });

        let mut tiles = vec![];

        let appstream_data = APPSTREAM_DATA_STATE.read();

        if !appstream_data.is_empty() {
            tiles = get_random_tiles(&appstream_data, 5);
        }

        let tiles = FactoryVecDeque::from_iter(tiles, adw::Carousel::new());

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
            CarouselInput::UpdateTiles(tiles) => {
                let mut guard = self.tiles.guard();
                guard.clear();
                for tile in tiles {
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

fn get_random_tiles(
    appstream_data: &HashMap<String, AppData>,
    amount: usize,
) -> Vec<CarouselTileInit> {
    use rand::seq::IteratorRandom;

    let mut rng = rand::rng();

    let mut tiles = appstream_data
        .iter()
        .filter_map(|(package, app_data)| {
            let name = app_data.name.as_ref()?.get("C")?.clone();

            let summary = app_data.summary.as_ref()?.get("C")?.clone();

            let icon = app_data
                .icon
                .as_ref()?
                .cached
                .as_ref()?
                .first()?
                .name
                .clone();

            let screenshot = app_data
                .screenshots
                .as_ref()?
                .iter()
                .find(|screenshot| screenshot.default.unwrap_or_default())
                .cloned()?
                .sourceimage?
                .url;

            let colors = app_data.branding.clone()?.colors;
            let color_dark = colors
                .iter()
                .find(|color| {
                    color
                        .scheme_preference
                        .as_ref()
                        .is_some_and(|scheme| scheme == &BrandingColorScheme::Dark)
                })?
                .value
                .clone();

            let color_light = colors
                .iter()
                .find(|color| {
                    color
                        .scheme_preference
                        .as_ref()
                        .is_some_and(|scheme| scheme == &BrandingColorScheme::Light)
                })?
                .value
                .clone();

            Some(CarouselTileInit {
                package: package.clone(),
                name,
                summary,
                icon,
                screenshot,
                color_dark,
                color_light,
            })
        })
        .sample(&mut rng, amount);
    tiles.shuffle(&mut rng);

    tiles
}
