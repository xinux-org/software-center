use relm4::{
    FactorySender, RelmWidgetExt, adw,
    factory::{DynamicIndex, FactoryComponent},
    gtk::{
        self,
        pango::EllipsizeMode,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
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
    type Init = CarouselTileInit;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            inline_css: "background-color: #114b91;",
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
            screenshot: "/nix/store/lqirp0agcqwmh52f3mjq6081ci2z8g68-nix-software-center-0.2.0/share/app-info/icons/nixos/128x128/bazaar_io.github.kolunmi.Bazaar.png".to_string(),
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {}
    }
}

// fn resize_image(path: &str) {}

// fn load_screenshots(package: &str, screenshot_url: String) {
//     debug!("Loading screenshots for package '{package}'");
//     let mut headers = reqwest::header::HeaderMap::new();
//     headers.insert(
//         reqwest::header::ACCEPT,
//         reqwest::header::HeaderValue::from_static("image/*"),
//     );

//     let client = reqwest::Client::builder()
//         .default_headers(headers)
//         .user_agent("nix-software-center")
//         .build()
//         .unwrap();

//     if let Ok(home) = std::env::var("HOME") {
//         let cache_dir = format!("{home}/.cache/nix-software-center/screenshots");

//         let sha = sha256::digest(&screenshot_url);

//         let client = client.clone();
//         let package = package.to_string();
//         let cache_dir = cache_dir.clone();

//         async move {
//             let path = format!("{cache_dir}/{sha}.png");
//             if Path::new(&path).exists() {
//                 // output_sender.send(PackageAsyncMessage::LoadScreenshot(package, i, path));
//             } else {
//                 if let Ok(path) = load_screenshot(&client, screenshot_url, cache_dir, sha).await {
//                     // output_sender.send(PackageAsyncMessage::LoadScreenshot(package, i, path));
//                 } else {
//                     // output_sender.send(PackageAsyncMessage::SetError(package, i));
//                 }
//             }
//         };
//     }
// }

// async fn load_screenshot(
//     client: &reqwest::Client,
//     url: String,
//     output_directory: String,
//     sha: String,
// ) -> anyhow::Result<String> {
//     debug!("Loading screenshot '{url}'");
//     tokio::time::sleep(Duration::from_millis(5)).await;
//     let path = format!("{output_directory}/{sha}.png");
//     if Path::new(&format!("{path}.png")).exists() {
//         Ok(path)
//     } else {
//         download_screenshot(client, url, output_directory, sha).await?;
//         Ok(path)
//     }
// }

// async fn download_screenshot(
//     client: &reqwest::Client,
//     url: String,
//     output_directory: String,
//     sha: String,
// ) -> anyhow::Result<String> {
//     debug!("Downloading screenshot '{url}'");
//     let path = format!("{output_directory}/{sha}.png");
//     let path_temp = format!("{output_directory}/{sha}");

//     let response = client.get(&url).send().await?;

//     if !response.status().is_success() {
//         anyhow::bail!("Screenshot could not be downloaded");
//     }

//     if !Path::new(&output_directory).exists() {
//         fs::create_dir_all(output_directory)?;
//     }

//     let mut file = File::create(&path_temp)?;
//     let bytes = response.bytes().await?;
//     let mut content = Cursor::new(bytes);
//     std::io::copy(&mut content, &mut file)?;

//     normalize_screenshot(&path_temp, &path)?;

//     Ok(path)
// }

// fn normalize_screenshot(old_path: &str, new_path: &str) -> anyhow::Result<()> {
//     debug!("Normalizing screenshot '{old_path}'");
//     let img = image::load(
//         BufReader::new(File::open(old_path)?),
//         image::ImageFormat::Png,
//     )
//     .or_else(|_| {
//         image::load(
//             BufReader::new(File::open(old_path)?),
//             image::ImageFormat::Jpeg,
//         )
//     })
//     .or_else(|_| {
//         image::load(
//             BufReader::new(File::open(old_path)?),
//             image::ImageFormat::WebP,
//         )
//     })
//     .or_else(|_| {
//         let image_data = BufReader::new(File::open(old_path)?);
//         let format = image::guess_format(image_data.buffer())?;
//         image::load(image_data, format)
//     })?;

//     let scaled = img.resize(640, 360, FilterType::Lanczos3);
//     let mut output = File::create(new_path)?;
//     scaled.write_to(&mut output, ImageFormat::Png)?;

//     if let Err(e) = fs::remove_file(old_path) {
//         warn!("Could not delete file {}: {}", old_path, e);
//     }

//     Ok(())
// }
