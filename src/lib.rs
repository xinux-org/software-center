pub mod config;
pub mod ui;
pub mod utils;

pub mod icon_names {
    pub use shipped::*;
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

static APPINFO: &str = "./result/share/app-info";
