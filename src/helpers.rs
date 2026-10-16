use crate::*;

use serde::{Serialize};
use std::path::PathBuf;
use native_dialog::{DialogBuilder};
use zip::ZipArchive;
use std::{io::Read};

pub fn create_default_profile<P: AsRef<std::path::Path>>(mods_folder_path: P) -> UResult<UnrusterProfile> {
    info!("Current profile directory is empty, current profile shall be of your current mods.");
    let mut default_profile = UnrusterProfile {profile_name: DEFAULT_PROFILE_NAME.to_string(), game_version: get_game_version()?, mods: vec![]};

    for file in std::fs::read_dir(mods_folder_path.as_ref())? {
        let entry= file?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("zip") {
            let mod_info: UnrusterModInfo = get_modinfo_from_zip(&path)?;
            if mod_info.is_locked == true {
                if !std::path::Path::new(CACHED_MODS_PATH).exists() {
                    std::fs::create_dir_all(CACHED_MODS_PATH)?;
                }
                let new_path = std::path::Path::new(CACHED_MODS_PATH).join(path.file_name().ok_or("Couldn't get filename")?);

                if !new_path.exists() {
                    std::fs::copy(path, new_path)?;
                }
            }
            default_profile.mods.push(mod_info);
        }
    }
    //let profile_path = std::path::Path::new(PROFILES_PATH).join(format!("{}.json", &default_profile.profile_name));
    //write_json(&default_profile, &profile_path)?;

    Ok(default_profile)
}

pub fn create_default_settings() -> UResult<UnrusterConfig> {
    info!("Config directory is empty, setting up configuration...");
    let mods_path = match dirs::data_dir() {
        Some(data_dir) => data_dir
            .join(VINTAGE_STORY_MODS_PATH)
            .to_string_lossy()
            .into_owned(),
        
        None => get_mod_directory()?.to_str().ok_or("Couldn't convert path into slice.")?.to_string()
    };

    let config = UnrusterConfig { mod_folder_path: mods_path.to_string(), active_profile_name: DEFAULT_PROFILE_NAME.to_string()};
    //let config_path = std::path::Path::new(CONFIG_FILE_PATH).join("default_config.json");
    //write_json(&config, &config_path)?;

    Ok(config)
}

pub fn get_game_version() -> UResult<String> {
    let logs_path = match dirs::data_dir() {
        Some(data_dir) => data_dir
            .join(VINTAGE_STORY_LOGS_PATH).join(VINTAGE_STORY_CLIENT_MAIN)
            .to_string_lossy()
            .into_owned(),
        
        None => get_log_path()?.to_str().ok_or("Couldn't convert path into slice.")?.to_string()
    };

    let log = std::fs::read_to_string(logs_path)?;

    let version = log
        .lines()
        .find_map(|line| {
            line.split_once("Game Version: ")
                .map(|(_, version)| version)
        })
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.strip_prefix('v'))
        .ok_or("Couldn't parse version from logs.")?;

    //info!("Game version: {:?}", version);

    Ok(version.to_string())
}

pub fn get_mod_directory() -> UResult<PathBuf> {
    let path = DialogBuilder::file()
    .set_location(&dirs::data_dir().ok_or("Data directory not found")?.join(VINTAGE_STORY_MODS_PATH))
    .set_title("Confirm your mod directory location")
    .open_single_dir()
    .show()?
    .ok_or("You went and cancelled folder selection smartass")?;

    Ok(path)
}

pub fn get_log_path() -> UResult<PathBuf> {
    let path = DialogBuilder::file()
    .set_location(&dirs::data_dir().ok_or("Data directory not found")?.join(VINTAGE_STORY_LOGS_PATH))
    .add_filter("Text file", ["txt"])
    .open_single_file()
    .show()?
    .ok_or("You went and cancelled folder selection smartass")?;

    Ok(path)
}

pub fn check_directory_contents<P: AsRef<std::path::Path>>(path: P, check_for: &str) -> UResult<DirectoryState> {
    let directory = path.as_ref();

    if !directory.exists() {
        info!("{:?} does not exist, creating it.", directory);
        std::fs::create_dir_all(directory)?;
    }

    let mut files_in_dir= std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|e| e.to_str()) == Some(check_for)
        });
    
    match (files_in_dir.next(), files_in_dir.next()) {
        (None, _) => Ok(DirectoryState::Empty),
        (Some(first), None) => Ok(DirectoryState::Single(first)),
        (Some(_), Some(_)) => Ok(DirectoryState::Multiple(std::fs::read_dir(directory)?)),
    }
}

pub fn get_modinfo_from_zip<P: AsRef<std::path::Path>>(path: P) -> UResult<UnrusterModInfo> {
    let file = std::fs::File::open(path.as_ref())?;
    let mut archive = ZipArchive::new(file)?;
    let mut zipped_mod_info = archive.by_name("modinfo.json")?;
    let mut modinfo_json = String::new();
    zipped_mod_info.read_to_string(&mut modinfo_json)?;

    let mod_info: UnrusterModInfo = json5::from_str(&modinfo_json)?;

    Ok(mod_info)
}

pub fn empty_directory<P: AsRef<std::path::Path>>(path: P) -> UResult<()> {
    let _ = std::fs::remove_dir_all(path.as_ref()); 
    std::fs::create_dir_all(path.as_ref())?;

    Ok(())
}

pub fn find_profile_by_name(name: &str) -> UResult<UnrusterProfile> {
    let path = std::path::Path::new(PROFILES_PATH).join(format!("{}.json", name));
    if !path.exists() {
        return Err(format!("Profile {} not found", name).into());
    }
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn find_cached_mod(modid: &str) -> UResult<std::path::PathBuf> {
    if !std::path::Path::new(CACHED_MODS_PATH).exists() {
        std::fs::create_dir_all(CACHED_MODS_PATH)?;
    }

    let entries = std::fs::read_dir(CACHED_MODS_PATH)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.path().extension().and_then(|e| e.to_str()) == Some("zip")
        });
    for entry in entries {
        let modinfo = get_modinfo_from_zip(entry.path())?;
        if modinfo.modid == modid {
            return Ok(entry.path())
        }
    }
    Err("Couldn't find the cached mod...".into())
}

pub fn write_json<S: Serialize, P: AsRef<std::path::Path>>(target: &S, path: P) -> UResult<()> {
    let file = std::fs::File::create(path.as_ref())?;
    serde_json::to_writer_pretty(file, target)?;
    Ok(())
}

pub fn user_querry(querry_msg: &str) -> UResult<u8> {
    info!("{}", querry_msg);

    let mut users_input = String::new();
    std::io::stdin().read_line(&mut users_input)?;

    let users_will: u8 = match users_input.trim().parse() {
        Ok(num) => num,
        Err(_) => u8::MAX
    };
    Ok(users_will)
}

pub fn user_interface(state: &mut UnrusterState) -> UResult<()> {
    info!("Welcome to the Unruster Mod Manager for Vintage Story!");
    info!("Current profile is {}", state.current_profile.profile_name);
    info!("The manager currently offers the following functionality:");

    loop {
        match user_querry("------------------------------\n1: Check mods for updates\n2: Sync profile to current mods\n3: Sync mods to current profile\n4: Create new profile\n5: Change profile\nElse: Quit")? {
            1 => {
                let updatables = state.run_update_check()?;

                match user_querry("Do you wish to update all your mods?\n1: Yes\nElse: No")? {
                    1 => {
                        info!("Updating all mods...");
                        state.update_all_mods(updatables)?;
                        info!("All mods up to date. Syncing profile to reflect new modlist...");
                        state.sync_profile_to_mods()?;
                        info!("Profile synced and mods are up to date :)");
                    }
                    _ => {
                        info!("Will not update mods...");
                        continue;
                    }
                }
            }
            2 => {
                info!("Updating profile to reflect current mods...");
                state.sync_profile_to_mods()?;
                info!("Profile updated.");
            }
            3 => {
                info!("Syncing mods to current profile...");
                state.sync_mods_to_profile()?;
                info!("Mods now reflect current profile.");
            }
            4 => {
                info!("Creating new profile...\nPlease, input the new profile's name:\n");
                let mut inputed_name = String::new();
                std::io::stdin().read_line(&mut inputed_name)?;

                inputed_name = inputed_name.trim().to_ascii_lowercase();
                info!("Name will be {}", inputed_name);

                if inputed_name.is_empty() {
                    info!("Name cannot be empty!");
                    continue;
                }

                info!("Creating new profile named {}...", inputed_name);
                state.make_new_profile(&inputed_name)?;

                match user_querry("Profile created! Do you wish to switch to the new profile?\n1: Yes\nElse: No")? {
                    1 => {
                        info!("Switching profiles...");
                        state.switch_current_profile(&inputed_name)?;
                        info!("Profile succesfully switched, syncing mods folder to new blank slate profile...");
                        state.sync_mods_to_profile()?;
                        info!("Mods folder cleared! You're now ready to design your brand new modlist :)");
                    }
                    _ => {
                        info!("Will switch profiles...");
                        continue;
                    }
                }
            }
            5 => {
                info!("You wish to switch profiles...");
                let names = state.list_available_profiles()?;
                info!("Select your profile...");
                state.interactive_switch_profile(names)?;
            }
            _ => {
                info!("Unruster is shutting down...");
                break;
            }
        }
    }

    Ok(())
}