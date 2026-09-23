use relm4::{
    Component, ComponentParts, ComponentSender, adw,
    factory::FactoryVecDeque,
    gtk::{self, prelude::*},
};

use crate::ui::explore::components::carousel_tile::CarouselTileModel;

#[derive(Debug)]
pub struct CarouselModel {
    tiles: FactoryVecDeque<CarouselTileModel>,
}

#[derive(Debug)]
pub enum CarouselInput {}

#[derive(Debug)]
pub enum CarouselOutput {}

pub struct CarouselInit {}

#[relm4::component(pub)]
impl Component for CarouselModel {
    type CommandOutput = ();
    type Input = CarouselInput;
    type Output = CarouselOutput;
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
                    // connect_page_changed[sender] => move |x, _| {
                    //     let n = adw::Carousel::n_pages(x);
                    //     let i = adw::Carousel::position(x) as u32;
                    //     if i == 0 && n == 1 {
                    //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Single));
                    //     } else if i == 0 {
                    //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::First));
                    //     } else if i == n - 1 {
                    //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
                    //     } else {
                    //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                    //     }
                    // },
                },
                add_overlay = &gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                    // #[watch]
                    // set_reveal_child: model.carousel_page != CarouselPage::First && model.carousel_page != CarouselPage::Single,
                    set_reveal_child: true,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Fill,
                    gtk::Button {
                        set_can_focus: false,
                        set_width_request: 60,
                        add_css_class: "flat",
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        set_icon_name: "go-previous-symbolic",
                        // connect_clicked[sender, scrnfactory] => move |_| {
                        //     let i = adw::Carousel::position(&scrnfactory) as u32;
                        //     if i > 0 {
                        //         let w = scrnfactory.nth_page(i-1);
                        //         scrnfactory.scroll_to(&w, true);
                        //     }
                        //     if i == 1 {
                        //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::First));
                        //     } else if i > 0 {
                        //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                        //     }
                        // }
                    }
                },
                add_overlay = &gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::Crossfade,
                    // #[watch]
                    // set_reveal_child: model.carousel_page != CarouselPage::Last && model.carousel_page != CarouselPage::Single,
                    set_reveal_child: true,
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Fill,
                    gtk::Button {
                        set_can_focus: false,
                        set_width_request: 60,
                        add_css_class: "flat",
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        set_icon_name: "go-next-symbolic",
                        // connect_clicked[sender, scrnfactory] => move |_| {
                        //     let i = adw::Carousel::position(&scrnfactory) as u32;
                        //     if i < scrnfactory.n_pages() -1 {
                        //         let w = scrnfactory.nth_page(i+1);
                        //         scrnfactory.scroll_to(&w, true);
                        //     }
                        //     let n = scrnfactory.n_pages();
                        //     if i == n - 2 {
                        //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
                        //     } else if i <= n - 2 {
                        //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Middle));
                        //     } else {
                        //         sender.input(PackageMessage::SetCarouselPage(CarouselPage::Last));
                        //     }
                        // }
                    }
                }
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut tiles = FactoryVecDeque::builder()
            .launch(adw::Carousel::new())
            .detach();

        let mut guard = tiles.guard();
        guard.push_back(());
        guard.push_back(());
        guard.push_back(());
        guard.push_back(());
        guard.drop();

        let model = Self { tiles };

        let tiles_factory = model.tiles.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
