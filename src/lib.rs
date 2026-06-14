use std::{cmp::Ordering, fs::File, path::PathBuf, str::FromStr};
use semver::Version;
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
    current_config: UnrusterConfig,
    current_game_version: Version
}

impl UnrusterState {
    pub fn new() -> Result<UnrusterState, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("UnrusterModManager/0.1.0")
            .build()?;

        let config: UnrusterConfig = match check_directory_contents(CONFIG_FILE_PATH, "json")? {
            DirectoryState::Empty => {
                create_default_settings()?
            }
            DirectoryState::Single(file) => {
                let path = file.path();
                let config_string = std::fs::read_to_string(path)?;
                let current_config: UnrusterConfig = serde_json::from_str(&config_string)?;

                println!("Current config has the following path: {}", &current_config.mod_folder_path);
                current_config
            }
            DirectoryState::Multiple => {
                return Err(format!("Current config directory contains more than 1 files, expected exactly 1").into());
            }
        };

        let mods_path = PathBuf::from(&config.mod_folder_path);
        let profile: UnrusterProfile = match check_directory_contents(CURRENT_PROFILE_PATH, "json")? {
            DirectoryState::Empty => {
                create_default_profile(&mods_path)?
            }
            DirectoryState::Single(file) => {
                let path = file.path();
                let profile_string = std::fs::read_to_string(path)?;
                let current_profile: UnrusterProfile = serde_json::from_str(&profile_string)?;

                println!("Current profile has the following mods:\n");
                for mods in &current_profile.mods {
                    println!("{}", mods.modid);
                }
                current_profile
            }
            DirectoryState::Multiple => {
                return Err(format!("Current profile directory contains more than 1 files, expected exactly 1").into());
            }
        };

        let version = Version::parse(&get_game_version()?)?;

        Ok(UnrusterState { 
            api_client: client, 
            current_profile: profile,
            current_config: config,
            current_game_version: version
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
        let download_path = std::path::Path::new(DOWNLOAD_TARGET_TEST).join(&release.filename);
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

    pub fn compare_mod_version<P: AsRef<std::path::Path>>(&self, file: P) -> Result<bool, Box<dyn std::error::Error>> {
        let mod_info: UnrusterModInfo = get_modinfo_from_zip(file.as_ref())?;
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

    pub fn sync_mods_to_profile(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.verify_profile_to_mods_sync()? {
            println!("Mods are already fully in sync with profile!");
            return Ok(());
        }

        if !matches!(check_directory_contents(DOWNLOAD_TARGET_TEST, "zip")?, DirectoryState::Empty) {
            println!("Completely deleting everything inside the mods folder");
            empty_directory(DOWNLOAD_TARGET_TEST)?;
        }

        for modid in &self.current_profile.mods {
            self.download_mod(&modid.modid, Some(semver::Version::from_str(&modid.version)?))?
        }

        Ok(())
    }

    fn verify_profile_to_mods_sync(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if let DirectoryState::Empty = check_directory_contents(DOWNLOAD_TARGET_TEST, "zip")? {
            return Ok(false);
        }

        let entries: Vec<_> = std::fs::read_dir(DOWNLOAD_TARGET_TEST)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "zip"))
            .collect();

        if entries.len() != self.current_profile.mods.len() {
            return Ok(false);
        }

        let profile_set: std::collections::HashSet<(String, String)> = self.current_profile.mods
            .iter()
            .map(|m| (m.modid.clone(), m.version.clone()))
            .collect();

        for item in entries {
            let path = item.path();
            let modinfo_in_folder = get_modinfo_from_zip(&path)?;

            let key = (modinfo_in_folder.modid, modinfo_in_folder.version);

            if !profile_set.contains(&key) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}