use unruster_mod_manager::{UnrusterState, UResult, info, LOGGER};

fn main() -> UResult<()> {
    let mut state = UnrusterState::new()?;

    info!("Welcome to the Unruster Mod Manager for Vintage Story!");
    info!("Current profile is {}", state.current_profile.profile_name);

    loop {
        info!("The manager currently offers the following functionality:");
        info!("1: Check mods for updates\n2: Sync profile to current mods\n3: Sync mods to current profile\n4: Create new profile\n5: Change profile\nElse: Quit\n");

        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input)?;

        let users_will: u8 = match user_input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                info!("Unruster is shutting down...");
                break
            }
        };

        match users_will {
            1 => {
                let updatables = state.run_update_check()?;
                info!("Do you wish to update all your mods?");
                info!("1: Yes\nElse: No\n");
                let mut user_input = String::new();
                std::io::stdin().read_line(&mut user_input)?;

                let users_will: u8 = match user_input.trim().parse() {
                    Ok(num) => num,
                    Err(_) => {
                        info!("Will not update mods...");
                        continue;
                    }
                };

                match users_will {
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
                info!("Creating new profile...");
                info!("Please, input the new profile's name:\n");
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
                info!("Profile created! Do you wish to switch to the new profile?");
                info!("1: Yes\nElse: No\n");
                
                let mut user_input = String::new();
                std::io::stdin().read_line(&mut user_input)?;

                let users_will: u8 = match user_input.trim().parse() {
                    Ok(num) => num,
                    Err(_) => {
                        info!("Will switch profiles...");
                        continue;
                    }
                };

                match users_will {
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