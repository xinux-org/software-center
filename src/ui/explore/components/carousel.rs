use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Cursor},
    path::Path,
};

use image::{ImageFormat, imageops::FilterType};
use log::{debug, warn};
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

#[derive(Debug)]
pub enum CarouselOutput {}

#[derive(Debug)]
pub enum CarouselCommandOutput {
    SetScreenshot(String, Option<String>),
}

#[relm4::component(pub)]
impl Component for CarouselModel {
    type CommandOutput = CarouselCommandOutput;
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
        init: Self::Init,
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

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CarouselCommandOutput::SetScreenshot(package, path) => {
                todo!()
            }
        }
    }
}

fn load_screenshots(
    sender: &ComponentSender<CarouselModel>,
    package: &str,
    screenshot_urls: HashMap<String, String>,
) {
    debug!("Loading screenshots for package '{package}'");
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("image/*"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .user_agent("nix-software-center")
        .build()
        .unwrap();

    if let Ok(home) = std::env::var("HOME") {
        let cache_dir = format!("{home}/.cache/nix-software-center/screenshots");

        for (i, (package, url)) in screenshot_urls.into_iter().enumerate() {
            let sha = sha256::digest(&url);

            let client = client.clone();
            let package = package.to_string();
            let cache_dir = cache_dir.clone();

            sender.command(move |output_sender, shutdown| {
                shutdown
                    .register(async move {
                        let path = format!("{cache_dir}/{sha}.png");
                        if let Ok(path) = load_screenshot(&client, url, cache_dir, sha).await {
                            // load
                            todo!()
                        } else {
                            // error
                            todo!()
                        }
                    })
                    .drop_on_shutdown()
            });
        }
    }
}

async fn load_screenshot(
    client: &reqwest::Client,
    url: String,
    output_directory: String,
    sha: String,
) -> anyhow::Result<String> {
    debug!("Loading screenshot '{url}'");
    let path = format!("{output_directory}/{sha}.png");
    let cropped_path = format!("{output_directory}/{sha}_cropped.png");
    let temp_path = format!("{output_directory}/{sha}_temp.png");

    if Path::new(&cropped_path).exists() {
        Ok(cropped_path)
    } else if Path::new(&path).exists() {
        crop_screenshot(&path, &cropped_path)?;

        Ok(cropped_path)
    } else {
        download_screenshot(client, &url, &temp_path).await?;
        normalize_screenshot(&temp_path, &path)?;
        crop_screenshot(&path, &cropped_path)?;

        Ok(cropped_path)
    }
}

async fn download_screenshot(
    client: &reqwest::Client,
    url: &str,
    path: &str,
) -> anyhow::Result<()> {
    debug!("Downloading screenshot '{url}'");

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Screenshot could not be downloaded");
    }

    let mut file = File::create(&path)?;
    let bytes = response.bytes().await?;
    let mut content = Cursor::new(bytes);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}

fn normalize_screenshot(old_path: &str, new_path: &str) -> anyhow::Result<()> {
    debug!("Normalizing screenshot '{old_path}'");
    let img = image::load(
        BufReader::new(File::open(old_path)?),
        image::ImageFormat::Png,
    )
    .or_else(|_| {
        image::load(
            BufReader::new(File::open(old_path)?),
            image::ImageFormat::Jpeg,
        )
    })
    .or_else(|_| {
        image::load(
            BufReader::new(File::open(old_path)?),
            image::ImageFormat::WebP,
        )
    })
    .or_else(|_| {
        let image_data = BufReader::new(File::open(old_path)?);
        let format = image::guess_format(image_data.buffer())?;
        image::load(image_data, format)
    })?;

    let scaled = img.resize(640, 360, FilterType::Lanczos3);
    let mut output = File::create(new_path)?;
    scaled.write_to(&mut output, ImageFormat::Png)?;

    if let Err(e) = fs::remove_file(old_path) {
        warn!("Could not delete file {}: {}", old_path, e);
    }

    Ok(())
}

fn crop_screenshot(uncropped: &str, cropped: &str) -> anyhow::Result<()> {
    let img = image::load(
        BufReader::new(File::open(uncropped)?),
        image::ImageFormat::Png,
    )?;
    let img = img.crop_imm(0, 0, 640, 200);

    let output = File::create(cropped)?;
    img.write_to(output, ImageFormat::Png)?;

    Ok(())
}
