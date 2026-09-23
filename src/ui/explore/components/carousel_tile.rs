use std::{
    fs::{self, File},
    io::{BufReader, Cursor},
    path::Path,
    time::Duration,
};

use image::{ImageFormat, imageops::FilterType};
use log::{debug, warn};
use relm4::{
    FactorySender, RelmWidgetExt, adw,
    factory::{DynamicIndex, FactoryComponent},
    gtk::{self, pango::EllipsizeMode, prelude::*},
};

use crate::APPINFO;

#[tracker::track]
#[derive(Debug)]
pub struct CarouselTileModel {
    name: String,
    summary: String,
    icon: String,
    screenshot: Option<String>,
    error: bool,
}

#[derive(Debug)]
pub enum CarouselTileOutput {}

pub struct CarouselTileInit {
    pub package: String,
    pub name: String,
    pub summary: String,
    pub icon: String,
    pub screenshot: String,
}

#[derive(Debug)]
pub enum CarouselTileCommandOutput {
    SetScreenshot(String),
    SetError,
}

#[relm4::factory(pub)]
impl FactoryComponent for CarouselTileModel {
    type ParentWidget = adw::Carousel;
    type Input = ();
    type Output = CarouselTileOutput;
    type Init = CarouselTileInit;
    type CommandOutput = CarouselTileCommandOutput;

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
                    set_valign: gtk::Align::Fill,
                    if self.screenshot.is_some() {
                        gtk::Box {
                            set_valign: gtk::Align::End,
                            set_halign: gtk::Align::Center,
                            set_vexpand: true,
                            gtk::Picture {
                                set_margin_top: 40,
                                #[watch]
                                set_filename: Some(self.screenshot.as_deref().unwrap_or_default()),
                            },
                        }
                    } else if !self.error {
                        gtk::Spinner {
                            set_spinning: true,
                        }
                    } else{
                        gtk::Image {
                            add_css_class: "error",
                            set_pixel_size: 128,
                            set_icon_name: Some("dialog-error-symbolic"),
                        }
                    }
                },
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
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

            let sha = sha256::digest(&init.screenshot);

            let client = client.clone();
            let cache_dir = cache_dir.clone();

            sender.command(move |output_sender, shutdown| {
                shutdown
                    .register(async move {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        let path = load_screenshot(&client, init.screenshot, cache_dir, sha).await;
                        if let Ok(path) = path {
                            output_sender.send(CarouselTileCommandOutput::SetScreenshot(path));
                        } else {
                            output_sender.send(CarouselTileCommandOutput::SetError);
                        }
                    })
                    .drop_on_shutdown()
            });
        }

        Self {
            name: init.name,
            summary: init.summary,
            icon: format!("{}/icons/nixos/128x128/{}", APPINFO, init.icon),
            screenshot: None,
            error: false,

            tracker: 0,
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, _sender: FactorySender<Self>) {
        match message {
            CarouselTileCommandOutput::SetScreenshot(path) => self.set_screenshot(Some(path)),
            CarouselTileCommandOutput::SetError => self.set_error(true),
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

    let mut file = File::create(path)?;
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
        warn!("Could not delete file {old_path}: {e}");
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
