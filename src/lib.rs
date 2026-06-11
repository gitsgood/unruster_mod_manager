use std::{cmp::Ordering, fs::File, io::Read, path::PathBuf, str::FromStr};
use semver::Version;
use zip::ZipArchive;
use native_dialog::{DialogBuilder};
use colored::Colorize;

pub mod json_structs;
pub mod constants_for_posterity;

pub use json_structs::*;
pub use constants_for_posterity::*;

#[allow(unused)]
pub struct UnrusterState {
    api_client: reqwest::blocking::Client,
    pub current_profile: UnrusterProfile,
}

impl UnrusterState {
    pub fn new() -> Result<UnrusterState, Box<dyn std::error::Error>> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("UnrusterModManager/0.1.0")
            .build()?;

        let current_profile_directory = PathBuf::from(UNRUSTER_CURRENT_PROFILE_PATH);

        if !current_profile_directory.exists() {
            println!("Directory does not exist, creating one.");
            std::fs::create_dir_all(UNRUSTER_CURRENT_PROFILE_PATH)?;
        } 

        let mut files_in_current_profile_dir= std::fs::read_dir(&current_profile_directory)?
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|e| e.to_str()) == Some("json")
            });

        let current_profile: UnrusterProfile = match (files_in_current_profile_dir.next(), files_in_current_profile_dir.next()) {
            (None, _) => {
                Self::create_default_profile()?
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

    fn create_default_profile() -> Result<UnrusterProfile, Box<dyn std::error::Error>> {
        println!("Current profile directory is empty, current profile shall be of your current mods.");
        let mut default_profile = UnrusterProfile {profile_name: "Default".to_string(), mods: vec![]};

        for file in std::fs::read_dir(get_mod_directory().ok_or("Path dialogue didn't do its thing")?)? {
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
        let profile_path = PathBuf::from(UNRUSTER_CURRENT_PROFILE_PATH).join("default_profile.json");

        let json = serde_json::to_string_pretty(&default_profile)?;
        std::fs::write(profile_path, json)?;

        Ok(default_profile)
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
}

#[allow(unused)]
pub fn download_mod(url:&str) -> Result<(), Box<dyn std::error::Error>> {
    let api_url = url;//"https://mods.vintagestory.at/api/mod/cbr";
    
    println!("Fetching mod metadata from Vintage Story API...");
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("VintageStoryModUpdater/0.0.1")
        .build()?;
        
    // Fetch and parse the nested API response
    let response: VSApiResponse = client.get(api_url).send()?.json()?;
    
    // Access the releases inside our renamed "mod_data" field
    if let Some(latest) = response.mod_data.releases.first() {
        println!("Latest version found: {}", latest.modversion);
        println!("Target file: {}", latest.filename);
        println!("Download URL: {}", latest.mainfile);
        
        println!("Downloading file...");
        let mut download_stream = client.get(&latest.mainfile).send()?;
        
        let mut local_file = File::create(&latest.filename)?;
        std::io::copy(&mut download_stream, &mut local_file)?;
        
        println!("Download complete! Saved as '{}'", latest.filename);
    } else {
        println!("No releases were found for this mod.");
    }
    
    Ok(())
}

pub fn check_latest(modid:&str) -> Result<String, Box<dyn std::error::Error>> {
    let api_url: String = VINTAGE_STORY_API_PREFIX.to_string() + modid;

    let client: reqwest::blocking::Client = reqwest::blocking::Client::builder()
    .user_agent("VintageStoryModUpdater/0.0.1")
    .build()?;

    let response: VSApiResponse = client.get(api_url).send()?.json()?;

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
    Ok(latest_version.to_string())
    /*
    if let Some(latest) = response.mod_data.releases.first() {
        println!("Latest version is: {}", latest.modversion);
        Ok(latest.modversion.clone())
    }
    else {
        Err("Couldn't find modapi".into())
    }*/
}

pub fn compare_mod_version(file: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file)?;
    let mut archive = ZipArchive::new(file)?;
    let mut zipped_mod_info = archive.by_name("modinfo.json")?;
    let mut modinfo_json = String::new();
    zipped_mod_info.read_to_string(&mut modinfo_json)?;

    let mod_info: UnrusterModInfo = json5::from_str(&modinfo_json)?;
    println!("Version of installed mod is: {}", mod_info.version);

    let current = Version::parse(&mod_info.version)?;
    let latest = Version::parse(&check_latest(&mod_info.modid)?)?;

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
    /*
    let latest_version = check_latest(&mod_info.modid)?;

    let mut current_ver_vec: Vec<u16> = current_version.split(&['.', '-']).map(|s| s.parse::<u16>()).collect::<Result<Vec<u16>, ParseIntError>>()?;
    let mut latest_ver_vec: Vec<u16> = latest_version.split(&['.', '-']).map(|s| s.parse::<u16>()).collect::<Result<Vec<u16>, ParseIntError>>()?;

    let max_len = current_ver_vec.len().max(latest_ver_vec.len());

    current_ver_vec.resize(max_len, 0);
    latest_ver_vec.resize(max_len, 0);

    match current_ver_vec.cmp(&latest_ver_vec) {
        Ordering::Greater => {
            println!("Current {} version is newer than the newest available ,':/", mod_info.modid);
            return Ok(false)
        }
        Ordering::Equal => {
            println!("{} is up to date :)", mod_info.modid);
            return Ok(false)
        }
        Ordering::Less => {
            println!("{} has an update available!", mod_info.modid);
            return Ok(true)
        }
    }
    */
}

#[allow(unused)]
pub fn get_mod_path() -> Option<PathBuf> {
    let path = DialogBuilder::file()
    //.set_location(&std::env::current_dir().unwrap())
    .set_location("~/AppData/Roaming/VintagestoryData/Mods")
    .add_filter("ZipArchive", ["zip"])
    .open_single_file()
    .show()
    .unwrap()?;

    Some(path)
}

pub fn get_mod_directory() -> Option<PathBuf> {
    let path = DialogBuilder::file()
    .set_title("Locate your mod directory in VintageDataStory")
    .open_single_dir()
    .show()
    .unwrap()?;

    Some(path)
}