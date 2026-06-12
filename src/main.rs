use std::str::FromStr;

use unruster_mod_manager::{UnrusterState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let current_file_path = get_mod_path().expect("Dialogue was useless you dolt");
    let data_dir = dirs::data_dir()
    .ok_or("Couldn't determine data directory")?;

    println!("{:?}", data_dir);

    /*
    std::fs::create_dir(unruster_mod_manager::DOWNLOAD_TARGET_TEST)?;

    let state = UnrusterState::new()?;
    for modid in &state.current_profile.mods {
        state.download_mod(&modid.modid, Some(semver::Version::from_str(&modid.version)?))?;
    }
    */
    /*
    let current_file_directory = get_mod_directory().expect("Dialogue was useless, AGAIN");

    for file in std::fs::read_dir(&current_file_directory)? {
        let entry= file?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("zip") {
            compare_mod_version(path.to_str().expect("Couldn't convert path to slice"))?;
        }
        println!("");
    }*/
    //let is_update_available = compare_mod_version(current_file_path.to_str().expect("Couldn't convert path to slice"))?;
    //download_mod("https://mods.vintagestory.at/api/mod/wool")
    Ok(())
}