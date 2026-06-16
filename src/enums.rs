use crate::{UnrusterModInfo};
use colored::Colorize;

pub enum DirectoryState {
    Empty,
    Single(std::fs::DirEntry),
    Multiple(std::fs::ReadDir),
}

#[derive(PartialEq, Eq, Hash)]
pub enum ModUpdateState {
    UpToDate,
    OutOfDate,
    NewerThanNew,
    Locked
}
impl ModUpdateState {
    pub fn should_it_update(&self) -> bool {
        match self {
            ModUpdateState::UpToDate => false,
            ModUpdateState::OutOfDate => true,
            ModUpdateState::NewerThanNew => false,
            ModUpdateState::Locked => false,
        }
    }

    pub fn print_update_situation(&self, modid: &str) {
        match self {
            ModUpdateState::UpToDate => {
                println!("{} is up to date :)", modid);
            },
            ModUpdateState::OutOfDate => {
                println!("{}{}", modid.green().on_bright_white(), " has an update available!".green().on_bright_white());
            },
            ModUpdateState::NewerThanNew => {
                println!("{}{}{}", "Current ".red().on_yellow(), modid.red().on_yellow(), " version is newer than the newest available ,':/".red().on_yellow());
            },
            ModUpdateState::Locked => {
                println!("{}{}", modid.purple().on_yellow(), " is marked as locked!".purple().on_yellow());
            }
        }
    }
}

pub enum CompareInput<'a> {
    Path(&'a std::path::Path),
    ModInfo(UnrusterModInfo)
}