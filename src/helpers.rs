use crate::{json_structs::*, constants_for_posterity::*, UResult, enums::{DirectoryState}};

use serde::{Serialize};
use std::path::PathBuf;
use native_dialog::{DialogBuilder};
use zip::ZipArchive;
use std::{io::Read};

pub fn create_default_profile<P: AsRef<std::path::Path>>(path: P) -> UResult<UnrusterProfile> {
    println!("Current profile directory is empty, current profile shall be of your current mods.");
    let mut default_profile = UnrusterProfile {profile_name: "Default".to_string(), game_version: get_game_version()?, mods: vec![]};

    for file in std::fs::read_dir(path.as_ref())? {
        let entry= file?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("zip") {
            let mod_info: UnrusterModInfo = get_modinfo_from_zip(&path)?;
            default_profile.mods.push(mod_info);
        }
    }
    let profile_path = std::path::Path::new(CURRENT_PROFILE_PATH).join("default_profile.json");
    write_json(&default_profile, &profile_path)?;

    Ok(default_profile)
}

pub fn create_default_settings() -> UResult<UnrusterConfig> {
    println!("Config directory is empty, setting up configuration...");
    let mods_path = match dirs::data_dir() {
        Some(data_dir) => data_dir
            .join(VINTAGE_STORY_MODS_PATH)
            .to_string_lossy()
            .into_owned(),
        
        None => get_mod_directory()?.to_str().ok_or("Couldn't convert path into slice.")?.to_string()
    };

    let config = UnrusterConfig { mod_folder_path: mods_path.to_string()};
    let config_path = std::path::Path::new(CONFIG_FILE_PATH).join("default_config.json");
    write_json(&config, &config_path)?;

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

    println!("Game version: {:?}", version);

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
        println!("{:?} does not exist, creating it.", directory);
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
        (Some(_), Some(_)) => Ok(DirectoryState::Multiple),
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

pub fn write_json<S: Serialize, P: AsRef<std::path::Path>>(target: &S, path: P) -> UResult<()> {
    let file = std::fs::File::create(path.as_ref())?;
    serde_json::to_writer_pretty(file, target)?;
    Ok(())
}