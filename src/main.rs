use unruster_mod_manager::{UnrusterState, UResult};

fn main() -> UResult<()> {
    let mut state = UnrusterState::new()?;

    println!("Welcome to the Unruster Mod Manager for Vintage Story!");

    loop {
        println!("The manager currently offers the following functionality:");
        println!("1: Check mods for updates\n2: Sync profile to current mods\n3: Sync mods to current profile\n4: Create new profile\n5: Change profile\nElse: Quit\n");

        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input)?;

        let users_will: u8 = match user_input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Unruster is shutting down...");
                break
            }
        };

        match users_will {
            1 => {
                let updatables = state.run_update_check()?;
                println!("Do you wish to update all your mods?");
                println!("1: Yes\nElse: No\n");
                let mut user_input = String::new();
                std::io::stdin().read_line(&mut user_input)?;

                let users_will: u8 = match user_input.trim().parse() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Will not update mods...");
                        continue;
                    }
                };

                match users_will {
                    1 => {
                        println!("Updating all mods...");
                        state.update_all_mods(updatables)?;
                        println!("All mods up to date. Syncing profile to reflect new modlist...");
                        state.sync_profile_to_mods()?;
                        println!("Profile synced and mods are up to date :)");
                    }
                    _ => {
                        println!("Will not update mods...");
                        continue;
                    }
                }
            }
            2 => {
                println!("Updating profile to reflect current mods...");
                state.sync_profile_to_mods()?;
                println!("Profile updated.");
            }
            3 => {
                println!("Syncing mods to current profile...");
                state.sync_mods_to_profile()?;
                println!("Mods now reflect current profile.");
            }
            4 => {
                println!("Creating new profile...");
                println!("Please, input the new profile's name:\n");
                let mut inputed_name = String::new();
                std::io::stdin().read_line(&mut inputed_name)?;

                inputed_name = inputed_name.trim().to_ascii_lowercase();
                println!("Name will be {}", inputed_name);

                if inputed_name.is_empty() {
                    println!("Name cannot be empty!");
                    continue;
                }

                println!("Creating new profile named {}...", inputed_name);
                state.make_new_profile(&inputed_name)?;
                println!("Profile created! Do you wish to switch to the new profile?");
                println!("1: Yes\nElse: No\n");
                
                let mut user_input = String::new();
                std::io::stdin().read_line(&mut user_input)?;

                let users_will: u8 = match user_input.trim().parse() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Will switch profiles...");
                        continue;
                    }
                };

                match users_will {
                    1 => {
                        println!("Switching profiles...");
                        state.switch_current_profile(&inputed_name)?;
                        println!("Profile succesfully switched, syncing mods folder to new blank slate profile...");
                        state.sync_mods_to_profile()?;
                        println!("Mods folder cleared! You're now ready to design your brand new modlist :)");
                    }
                    _ => {
                        println!("Will switch profiles...");
                        continue;
                    }
                }
            }
            5 => {
                println!("You wish to switch profiles...");
                let names = state.list_available_profiles()?;
                println!("Select your profile...");
                state.interactive_switch_profile(names)?;
            }
            _ => {
                println!("Unruster is shutting down...");
                break;
            }
        }
    }

    Ok(())
}