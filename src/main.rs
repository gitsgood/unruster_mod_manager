use unruster_mod_manager::{UResult, UnrusterState, user_interface};

fn main() -> UResult<()> {
    let mut state = UnrusterState::new()?;

    user_interface(&mut state)?;

    Ok(())
}