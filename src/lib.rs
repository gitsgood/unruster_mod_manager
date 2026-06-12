use std::{cmp::Ordering, fs::File, io::Read, str::FromStr};
use semver::Version;
use zip::ZipArchive;
use colored::Colorize;

pub mod json_structs;
pub mod constants_for_posterity;
pub mod helpers;

pub use json_structs::*;
pub use constants_for_posterity::*;
pub use helpers::*;

#[allow(unused)]
pub struct UnrusterState {
    api_client: reqwest::blocking::Client,
    pub current_profile: UnrusterProfile,
    config: UnrusterConfig
}

impl UnrusterState {
    pub fn new() -> Result<UnrusterState, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("UnrusterModManager/0.1.0")
            .build()?;

        let current_profile: UnrusterProfile = match check_directory_contents(CURRENT_PROFILE_PATH)? {
            (None, _) => {
                create_default_profile()?
            }
            (Some(first), None) => {
                let path = first.path();
                let profile_string = std::fs::read_to_string(path)?;
                let current_profile: UnrusterProfile = serde_json::from_str(&profile_string)?;

                println!("Current profile has the following mods:\n");
                for mods in &current_profile.mods {
                    println!("{}", mods.modid);
                }
                current_profile
            }
            _ => {
                return Err(format!("Current profile directory contains more than 1 files, expected exactly 1").into());
            }
        };

        let config: UnrusterConfig = match check_directory_contents(CONFIG_FILE_PATH)? {
            (None, _) => {
                create_default_profile()?
            }
            (Some(first), None) => {
                let path = first.path();
                let profile_string = std::fs::read_to_string(path)?;
                let current_profile: UnrusterProfile = serde_json::from_str(&profile_string)?;

                println!("Current profile has the following mods:\n");
                for mods in &current_profile.mods {
                    println!("{}", mods.modid);
                }
                current_profile
            }
            _ => {
                return Err(format!("Current profile directory contains more than 1 files, expected exactly 1").into());
            }
        };

        Ok(UnrusterState { 
            api_client: client, 
            current_profile: current_profile 
        })
    }

    pub fn download_mod(&self, modid: &str, version: Option<Version>) -> Result<(), Box<dyn std::error::Error>> {
        let api_url = format!("{}{}", VINTAGE_STORY_API_PREFIX, modid);

        let response: VSApiResponse = self.api_client.get(api_url).send()?.json()?;

        let version_to_download = match version {
            Some(v) => v,
            None => self.check_latest(modid)?
        };

        let release = response
            .mod_data
            .releases
            .iter()
            .find(|r| {
                if let Ok(v) = Version::from_str(&r.modversion) {
                    v == version_to_download
                } else {
                    false
                }
            })
            .ok_or(format!("Mod '{}' does not contain version {:?}", modid, version_to_download))?;

        let mut download_stream = self.api_client.get(&release.mainfile).send()?.error_for_status()?;
        let download_path = format!("{}{}{}", DOWNLOAD_TARGET_TEST, "/" , &release.filename);
        let mut downloaded_mod_file = File::create(&download_path)?;

        std::io::copy(&mut download_stream, &mut downloaded_mod_file)?;

        Ok(())
    }

    fn check_latest(&self, modid: &str) -> Result<Version, Box<dyn std::error::Error>> {
        let api_url = format!("{}{}", VINTAGE_STORY_API_PREFIX, modid);

        let response: VSApiResponse = self.api_client.get(api_url).send()?.json()?;

        let mut releases = response.mod_data.releases.into_iter();

        let first = releases.next().ok_or("No releases found")?;

        let mut latest_version = Version::parse(&first.modversion)?;

        for release in releases {
            //println!("{}", release.modversion);
            let this_version = Version::parse(&release.modversion)?;

            if this_version > latest_version {
                latest_version = this_version;
            }
        }
        println!("{}", latest_version.to_string().yellow().on_red());
        Ok(latest_version)
    }

    pub fn compare_mod_version(&self, file: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file)?;
        let mut archive = ZipArchive::new(file)?;
        let mut zipped_mod_info = archive.by_name("modinfo.json")?;
        let mut modinfo_json = String::new();
        zipped_mod_info.read_to_string(&mut modinfo_json)?;

        let mod_info: UnrusterModInfo = json5::from_str(&modinfo_json)?;
        println!("Version of installed mod is: {}", mod_info.version);

        let current = Version::parse(&mod_info.version)?;
        let latest = &self.check_latest(&mod_info.modid)?;

        match current.cmp(&latest) {
            Ordering::Greater => {
                println!("{}{}{}", "Current ".red().on_yellow(), mod_info.modid.red().on_yellow(), " version is newer than the newest available ,':/".red().on_yellow());
                return Ok(false)
            }
            Ordering::Equal => {
                println!("{} is up to date :)", mod_info.modid);
                return Ok(false)
            }
            Ordering::Less => {
                println!("{}{}", mod_info.modid.green().on_bright_white(), " has an update available!".green().on_bright_white());
                return Ok(true)
            }
        }
    }
}