use std::{cmp::Ordering, fs::File, str::FromStr, io::Write};
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
        let download_path = std::path::Path::new(&self.current_config.mod_folder_path).join(&release.filename);
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
            println!("WARNING!\nProfile expected game version: {}\nBut current game version is: {}", &self.current_profile.game_version, &self.current_game_version)
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

    pub fn run_update_check(&self) -> UResult<std::collections::HashSet<(ModUpdateState, String)>> {
        let results = self.compare_all_mod_versions()?;
        
        println!("\n--- Mod Update Report ---\n");
        for (state, modid) in &results {
            state.print_update_situation(modid);
        }
        
        let update_count = results.iter().filter(|(s, _)| s.should_it_update()).count();
        println!("--------------------------");
        println!("Total updates available: {}", update_count);
        
        Ok(results)
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

        std::fs::create_dir_all(CACHED_MODS_PATH)?;

        let entries = std::fs::read_dir(&self.current_config.mod_folder_path)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "zip"));

        for entry in entries {
            let path = entry.path();

            let mod_info = get_modinfo_from_zip(&path)?;

            let is_locked = self.current_profile.mods
                .iter()
                .find(|m| m.modid == mod_info.modid)
                .map(|m| m.is_locked)
                .unwrap_or(false);

            if is_locked {
                let file_name = path.file_name().ok_or("Invalid filename")?;
                let cache_path = std::path::Path::new(CACHED_MODS_PATH).join(file_name);

                if !cache_path.exists() {
                    std::fs::copy(&path, cache_path)?;
                    println!("Backed up locked mod: {}", mod_info.modid);
                }
            } 
            else {
                std::fs::remove_file(path)?;
            }
        }

        for modid in &self.current_profile.mods {
            if !modid.is_locked {
                self.download_mod(&modid.modid, Some(semver::Version::from_str(&modid.version)?))?
            }
            else {
                let mod_name = format!("{}.zip", modid.name);
                let current_path = std::path::Path::new(&self.current_config.mod_folder_path).join(&mod_name);

                if !current_path.exists() {
                    let cached_mod = find_cached_mod(&modid.modid)?;
                    let new_path = std::path::Path::new(&self.current_config.mod_folder_path).join(cached_mod.file_name().unwrap());
                    std::fs::copy(cached_mod, new_path)?;
                }
            }
        }

        Ok(())
    }

    pub fn update_all_mods(&mut self, updatables: std::collections::HashSet<(ModUpdateState, String)>) -> UResult<()> {
        for (update_state, modid) in &updatables {
            if update_state.should_it_update() {
                self.download_mod(modid, Some(self.check_latest(modid)?))?;
            }
        }
        Ok(())
    }

    pub fn list_available_profiles(&self) -> UResult<Vec<String>> {
        let profiles_folder = std::fs::read_dir(PROFILES_PATH)?
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|e| e.to_str()) == Some("json")
            });
        
        let mut found_any = false;
        let mut names_list = vec![];

        for file in profiles_folder {
            if let Some(stem) = file.path().file_stem() {
                let profile_name = stem.to_string_lossy();

                if profile_name == self.current_profile.profile_name {
                    continue;
                }

                println!("- {}", profile_name);
                names_list.push(profile_name.into_owned());
                found_any = true;
            }
        }

        if !found_any {
            println!("No other profiles exist...\n");
        }

        Ok(names_list)
    }

    pub fn interactive_switch_profile(&mut self, profiles: Vec<String>) -> UResult<()> {
        let available_profiles = profiles;

        if available_profiles.is_empty() {
            println!("No other profiles available to switch to.");
            return Ok(());
        }

        loop {
            //println!("\n--- Select a Profile ---");
            for (i, name) in available_profiles.iter().enumerate() {
                println!("{}: {}", i + 1, name);
            }
            println!("Else: Cancel");
            print!("Enter number: ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();

            /*if input == "q" {
                println!("Switch cancelled.");
                return Ok(());
            }*/

            match input.parse::<usize>() {
                Ok(num) if num > 0 && num <= available_profiles.len() => {
                    let selected_name = &available_profiles[num - 1];

                    self.switch_current_profile(selected_name)?;
                    return Ok(());
                }
                _ => {
                    println!("Switch cancelled.");
                    return Ok(());
                }
            }
        }
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

        if mod_info.is_locked {
            return Ok(ModUpdateState::Locked)
        }

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
        if let DirectoryState::Empty = check_directory_contents(&self.current_config.mod_folder_path, "zip")? {
            return Ok(false);
        }

        let entries: Vec<_> = std::fs::read_dir(&self.current_config.mod_folder_path)?
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