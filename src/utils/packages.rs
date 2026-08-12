use anyhow::Result;
use chrono::{DateTime, Utc, serde::ts_seconds_option};
use flate2::bufread::GzDecoder;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    self,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
};

use crate::APPINFO;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum StrOrVec {
    Single(String),
    List(Vec<String>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum Platform {
    Single(String),
    List(Vec<String>),
    ListList(Vec<Vec<String>>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(untagged)]
pub enum LicenseEnum {
    Single(License),
    List(Vec<License>),
    SingleStr(String),
    VecStr(Vec<String>),
    Mixed(Vec<LicenseEnum>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct License {
    pub free: Option<bool>,
    #[serde(rename = "fullName")]
    pub fullname: Option<String>,
    #[serde(rename = "spdxId")]
    pub spdxid: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct PkgMaintainer {
    pub email: Option<String>,
    pub github: Option<String>,
    pub matrix: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppData {
    #[serde(rename = "Type")]
    pub metatype: String,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Package")]
    pub package: String,
    #[serde(rename = "Name")]
    pub name: Option<HashMap<String, String>>,
    #[serde(rename = "Description")]
    pub description: Option<HashMap<String, String>>,
    #[serde(rename = "Summary")]
    pub summary: Option<HashMap<String, String>>,
    #[serde(rename = "Url")]
    pub url: Option<AppUrl>,
    #[serde(rename = "Icon")]
    pub icon: Option<AppIconList>,
    #[serde(rename = "Launchable")]
    pub launchable: Option<AppLaunchable>,
    #[serde(rename = "Provides")]
    pub provides: Option<AppProvides>,
    #[serde(rename = "Screenshots")]
    pub screenshots: Option<Vec<AppScreenshot>>,
    #[serde(rename = "Categories")]
    pub categories: Option<Vec<String>>,
    #[serde(rename = "Releases")]
    pub releases: Option<Vec<AppRelease>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppUrl {
    /// Should be a link to the upstream homepage for the component.
    pub homepage: Option<String>,
    /// Should point to the software's bug tracking system, for users to report new bugs.
    pub bugtracker: Option<String>,
    /// Should link a FAQ page for this software, to answer some of the most-asked questions in detail, something which you cannot do in the component's description.
    pub faq: Option<String>,
    /// Should provide a web link to an online user's reference, a software manual or help page.
    pub help: Option<String>,
    /// URLs of this type should point to a webpage showing information on how to donate to the described software project.
    pub donation: Option<String>,
    /// URLs of this type should point to a webpage where users can submit or modify translations of the upstream project.
    pub translate: Option<String>,
    /// URLs of this type should allow the user to contact the developer. This could for example be an HTTPS URL to an online form or a page describing how to contact the developer.
    pub contact: Option<String>,
    #[serde(rename = "vcs-browser")]
    /// URLs of this type should point to a webpage on which the user can browse the sourcecode.
    pub vcs_browser: Option<String>,
    /// URLs of this type should point to a webpage showing information on how to contribute to the described software project.
    pub contribute: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppIconList {
    pub cached: Option<Vec<AppIcon>>,
    pub stock: Option<String>,
    // TODO: add support for other icon types
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AppIcon {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppLaunchable {
    #[serde(rename = "desktop-id")]
    pub desktopid: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppProvides {
    pub binaries: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
    pub mediatypes: Option<Vec<String>>,
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppScreenshot {
    pub default: Option<bool>,
    pub thumbnails: Option<Vec<String>>,
    #[serde(rename = "source-image")]
    pub sourceimage: Option<AppScreenshotImage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppScreenshotImage {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppRelease {
    pub version: Option<String>,
    #[serde(default, rename = "type")]
    pub release_type: ReleaseType,
    pub date: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option", rename = "unix-timestamp")]
    pub timestamp: Option<DateTime<Utc>>,
    pub description: Option<HashMap<String, String>>,
    pub url: Option<ReleaseUrl>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    #[default]
    Stable,
    Development,
    Snapshot,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReleaseUrl {
    pub details: Option<String>,
}

pub fn appsteamdata() -> Result<HashMap<String, AppData>> {
    let appdata = File::open(format!("{}/xmls/nixos_x86_64_linux.yml.gz", APPINFO))?;
    let appreader = BufReader::new(appdata);
    let mut d = GzDecoder::new(appreader);
    let mut s = String::new();
    d.read_to_string(&mut s)?;
    let mut files = s.split("\n---\n").collect::<Vec<_>>();
    files.remove(0);

    let mut out = HashMap::new();

    for f in files {
        if let Ok(appstream) = serde_yaml::from_str::<AppData>(f) {
            out.insert(appstream.package.to_string(), appstream);
        } else {
            warn!("Failed to parse some appstream data");
        }
    }
    Ok(out)
}
