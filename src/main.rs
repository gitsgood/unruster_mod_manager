//use std::str::FromStr;

use unruster_mod_manager::{UnrusterState, UResult};

fn main() -> UResult<()> {
    let data_dir = dirs::data_dir()
    .ok_or("Couldn't determine data directory")?;

    println!("{:?}", data_dir);

    let _state = UnrusterState::new()?;

    _state.run_update_check()?;

    Ok(())
}