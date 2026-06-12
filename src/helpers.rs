use crate::json_structs::*;
use crate::constants_for_posterity::*;

use std::path::PathBuf;
use native_dialog::{DialogBuilder};
use zip::ZipArchive;
use std::{io::Read};

pub fn create_default_profile() -> Result<UnrusterProfile, Box<dyn std::error::Error>> {
    println!("Current profile directory is empty, current profile shall be of your current mods.");
    let mut default_profile = UnrusterProfile {profile_name: "Default".to_string(), mods: vec![]};

    for file in std::fs::read_dir(get_mod_directory()?)? {
        let entry= file?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("zip") {
            let file = std::fs::File::open(path)?;
            let mut archive = ZipArchive::new(file)?;
            let mut zipped_mod_info = archive.by_name("modinfo.json")?;
            let mut modinfo_json = String::new();
            zipped_mod_info.read_to_string(&mut modinfo_json)?;

            let mod_info: UnrusterModInfo = json5::from_str(&modinfo_json)?;
            default_profile.mods.push(mod_info);
        }
    }
    let profile_path = PathBuf::from(CURRENT_PROFILE_PATH).join("default_profile.json");

    let json = serde_json::to_string_pretty(&default_profile)?;
    std::fs::write(profile_path, json)?;

    Ok(default_profile)
}

pub fn create_default_settings() -> Result<UnrusterConfig, Box<dyn std::error::Error>> {
    println!("Config directory is empty, setting up configuration...");
    let mut default_config = UnrusterConfig {
        mod_folder_path: String::from(dirs::data_dir()
            .ok_or("Couldn't find the data directory")?
            .join(VINTAGE_STORY_MODS_PATH).to_str()
            .ok_or("Couldn't build the path")?)
        };
}

pub fn get_mod_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = DialogBuilder::file()
    .set_title("Locate your mod directory in VintageDataStory")
    .open_single_dir()
    .show()?
    .unwrap();

    Ok(path)
}

pub fn check_directory_contents(path: &str) -> Result<(std::option::Option<std::fs::DirEntry>, std::option::Option<std::fs::DirEntry>), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(path);

    if !directory.exists() {
        println!("{} does not exist, creating it.", &path);
        std::fs::create_dir_all(path)?;
    }

    let mut files_in_dir= std::fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|e| e.to_str()) == Some("json")
        });
    
    Ok((files_in_dir.next(), files_in_dir.next()))
}