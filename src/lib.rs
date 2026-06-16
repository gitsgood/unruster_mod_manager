use std::{cmp::Ordering, fs::File, str::FromStr};
use semver::Version;
use rayon::prelude::*;

pub mod json_structs;
pub mod constants_for_posterity;
pub mod helpers;
pub mod enums;

pub use json_structs::*;
pub use constants_for_posterity::*;
pub use helpers::*;
pub use enums::*;

pub type UResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct UnrusterState {
    api_client: reqwest::blocking::Client,
    current_profile: UnrusterProfile,
    current_config: UnrusterConfig,
    current_game_version: Version
}

impl UnrusterState {
    pub fn new() -> UResult<UnrusterState> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("UnrusterModManager/0.1.0")
            .build()?;

        let mut config: UnrusterConfig = match check_directory_contents(CONFIG_FILE_PATH, "json")? {
            DirectoryState::Empty => {
                create_default_settings()?
            }
            DirectoryState::Single(file) => {
                let path = file.path();
                let config_string = std::fs::read_to_string(path)?;
                let current_config: UnrusterConfig = serde_json::from_str(&config_string)?;

                println!("Config has the following mod path: {}", &current_config.mod_folder_path);
                current_config
            }
            DirectoryState::Multiple(_) => {
                return Err(format!("Current config directory contains more than 1 json files, expected exactly 1").into());
            }
        };

        let profile: UnrusterProfile = match check_directory_contents(PROFILES_PATH, "json")? {
            DirectoryState::Empty => {
                create_default_profile(&config.mod_folder_path)?
            }
            DirectoryState::Single(file) => {
                let path = file.path();
                let profile_string = std::fs::read_to_string(path)?;
                let current_profile: UnrusterProfile = serde_json::from_str(&profile_string)?;

                println!("Current profile has the following mods:\n");
                for mods in &current_profile.mods {
                    println!("{}", mods.modid);
                }
                config.active_profile_name = current_profile.profile_name.clone();

                current_profile
            }
            DirectoryState::Multiple(_) => {
                let profile = match find_profile_by_name(&config.active_profile_name) {
                    Ok(found_profile) => found_profile,
                    Err(_) => {
                        println!("Couldn't find profile named {}, reverting to default.", &config.active_profile_name);
                        config.active_profile_name = DEFAULT_PROFILE_NAME.to_string();
                        create_default_profile(&config.mod_folder_path)?
                    } 
                };
                profile
            }
        };

        let version = Version::parse(&get_game_version()?)?;

        let config_path = std::path::Path::new(CONFIG_FILE_PATH).join("config.json");
        write_json(&config, config_path)?;

        let profile_path = std::path::Path::new(PROFILES_PATH).join(format!("{}.json", &profile.profile_name));
        write_json(&profile, profile_path)?;

        Ok(UnrusterState { 
            api_client: client, 
            current_profile: profile,
            current_config: config,
            current_game_version: version
        })
    }

    pub fn download_mod(&self, modid: &str, version: Option<Version>) -> UResult<()> {
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

    pub fn switch_current_profile(&mut self, profile_name: &str) -> UResult<()> {
        let new_profile = find_profile_by_name(profile_name)?;

        self.current_config.active_profile_name = profile_name.to_string();

        let config_path = std::path::Path::new(CONFIG_FILE_PATH).join("config.json");
        write_json(&self.current_config, config_path)?;

        self.current_profile = new_profile;

        println!("Switched to profile: {}", profile_name);
        if self.current_game_version != Version::parse(&self.current_profile.game_version)? {
            println!("Profile expected game version: {}\nBut current game version is: {}", &self.current_profile.game_version, &self.current_game_version)
        }
        Ok(())
    }

    pub fn make_new_profile(&mut self, new_name: &str) -> UResult<()> {
        let new_profile = UnrusterProfile {profile_name: new_name.to_string(), game_version: get_game_version()?, mods: vec![]};

        self.current_profile = new_profile;

        let profile_path = std::path::Path::new(PROFILES_PATH).join(format!("{}.json", &self.current_profile.profile_name));
        write_json(&self.current_profile, profile_path)?;

        Ok(())
    }

    pub fn run_update_check(&self) -> UResult<()> {
        let results = self.compare_all_mod_versions()?;
        
        println!("\n--- Mod Update Report ---\n");
        for (state, modid) in &results {
            state.print_update_situation(modid);
        }
        
        let update_count = results.iter().filter(|(s, _)| s.should_it_update()).count();
        println!("--------------------------");
        println!("Total updates available: {}", update_count);
        
        Ok(())
    }

    pub fn sync_profile_to_mods(&mut self) -> UResult<()> {
        let mods_folder: Vec<std::fs::DirEntry> = std::fs::read_dir(&self.current_config.mod_folder_path)?
            .filter_map(Result::ok)
            .filter(|e| {
            e.path()
                .extension()
                .and_then(|e| e.to_str()) == Some("zip")
            })
            .collect();
        
        self.current_profile.mods.clear();
        self.current_profile.mods.reserve_exact(mods_folder.len());

        for file in mods_folder {
            let mod_info: UnrusterModInfo = get_modinfo_from_zip(&file.path())?;
            self.current_profile.mods.push(mod_info);

        }

        let profile_path = std::path::Path::new(PROFILES_PATH).join(format!("{}.json", &self.current_profile.profile_name));
        write_json(&self.current_profile, profile_path)?;

        Ok(())
    }

    pub fn sync_mods_to_profile(&self) -> UResult<()> {
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

    fn check_latest(&self, modid: &str) -> UResult<Version> {
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
        //println!("{}", latest_version.to_string().yellow().on_red());
        Ok(latest_version)
    }

    fn compare_all_mod_versions(&self) -> UResult<std::collections::HashSet<(ModUpdateState, String)>> {
        let path = std::path::Path::new(&self.current_config.mod_folder_path);
        let directory: Vec<_> = std::fs::read_dir(path)?            
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "zip"))
            .collect();

        let updatables: std::collections::HashSet<(ModUpdateState, String)> = directory
            .par_iter()
            .map(|entry| {
                let entry_info = get_modinfo_from_zip(entry.path())?;
                let update_state = self.compare_mod_version(CompareInput::ModInfo(entry_info.clone()))?;
                Ok((update_state, entry_info.modid))
            })
            .collect::<UResult<std::collections::HashSet<(ModUpdateState, String)>>>()?;

        Ok(updatables)
    }

    fn compare_mod_version(&self, file: CompareInput) -> UResult<ModUpdateState> {
        let mod_info: UnrusterModInfo = match file {
            CompareInput::Path(path) => get_modinfo_from_zip(path)?,
            CompareInput::ModInfo(this) => this
        };
        //println!("Version of installed mod is: {}", mod_info.version);

        let current = Version::parse(&mod_info.version)?;
        let latest = &self.check_latest(&mod_info.modid)?;

        match current.cmp(&latest) {
            Ordering::Greater => {
                return Ok(ModUpdateState::NewerThanNew)
            }
            Ordering::Equal => {
                return Ok(ModUpdateState::UpToDate)
            }
            Ordering::Less => {
                return Ok(ModUpdateState::OutOfDate)
            }
        }
    }

    fn verify_profile_to_mods_sync(&self) -> UResult<bool> {
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
            .par_iter()
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