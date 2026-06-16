use serde::{Deserialize, Serialize};

/// Represent the release object containing the actual mod information
#[derive(Deserialize, Debug)]
pub struct Release {
    pub mainfile: String,   // What the client needs to ask for in order to actually download the mod
    pub filename: String,   // The filename, so that we can actually give it a name when it's downloaded (e.g., "CBR-1.0.0.zip")
    pub modversion: String, // The version of the mod
}

/// Represent the nested object inside the top-level api response
#[derive(Deserialize, Debug)]
pub struct ModReleases {
    pub releases: Vec<Release>,
}

/// Represent the top-level API response
#[derive(Deserialize, Debug)]
pub struct VSApiResponse {
    //pub statuscode: String,
    // Renamed "mod" from the JSON to "mod_data" because "mod" is a reserved keyword in Rust
    #[serde(rename = "mod")]
    pub mod_data: ModReleases,
}

/// Represent what we care about here, being version and modid. 
/// We can then make our own jsons keeping track of what we need later for profiles and whatnot based on this
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnrusterModInfo {
    #[serde(alias = "ModID", alias = "modID", alias = "modId")]
    pub modid: String,
    #[serde(alias = "Version")]
    pub version: String,
}

/// Represent, lo and behold, a mod profile
#[derive(Deserialize, Serialize, Debug)]
pub struct UnrusterProfile {
    pub profile_name: String,
    pub game_version: String,
    pub mods: Vec<UnrusterModInfo>
}

/// Might eventually put useful info in there.
#[derive(Deserialize, Serialize, Debug)]
pub struct UnrusterConfig {
    pub mod_folder_path: String,
    pub active_profile_name: String
}