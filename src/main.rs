use eframe::egui;
use unruster_mod_manager::{UnrusterState, UResult};
use std::sync::{Arc, Mutex};
use std::thread;

struct UnrusterApp {
    // We wrap state in Arc<Mutex<>> so we can access it from background threads
    state: Arc<Mutex<UnrusterState>>,
    
    // GUI State
    new_profile_name: String,
    status_message: String,
    is_loading: bool,
    available_profiles: Vec<String>,
}

impl UnrusterApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize state (Note: this might block for a moment on startup)
        let state = UnrusterState::new().expect("Failed to initialize Unruster State");
        
        Self {
            state: Arc::new(Mutex::new(state)),
            new_profile_name: String::new(),
            status_message: "Ready".to_string(),
            is_loading: false,
            available_profiles: vec![],
        }
    }

    // Helper to run heavy tasks in a background thread so the GUI doesn't freeze
    fn run_task<F>(&mut self, task: F) 
    where 
        F: FnOnce(&mut UnrusterState) -> UResult<()> + Send + 'static 
    {
        self.is_loading = true;
        let state_clone = Arc::clone(&self.state);
        
        // This is a hacky way to update the status message from a thread.
        // In a real app, you'd use a channel (mpsc) to send messages back to the GUI.
        thread::spawn(move || {
            let mut state = state_clone.lock().unwrap();
            if let Err(e) = task(&mut state) {
                eprintln!("Error executing task: {}", e);
            }
        });
    }
}

impl eframe::App for UnrusterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Unruster Mod Manager");
            ui.label(format!("Current Profile: {}", state.current_profile.profile_name));
            ui.separator();

            // --- MOD MANAGEMENT SECTION ---
            ui.collapsing("Mod Management", |ui| {
                if ui.button("Check for Updates").clicked() {
                    // For simplicity, we just run the check. 
                    // In a full app, you'd store the result in the App struct to display in a list.
                    let _ = state.run_update_check();
                    self.status_message = "Update check completed (see console)".to_string();
                }

                if ui.button("Sync Profile to Mods").clicked() {
                    // We use a clone of the state for the thread
                    let state_arc = Arc::clone(&self.state);
                    self.is_loading = true;
                    thread::spawn(move || {
                        let mut s = state_arc.lock().unwrap();
                        let _ = s.sync_profile_to_mods();
                    });
                    self.status_message = "Syncing profile...".to_string();
                }

                if ui.button("Sync Mods to Profile").clicked() {
                    let state_arc = Arc::clone(&self.state);
                    self.is_loading = true;
                    thread::spawn(move || {
                        let s = state_arc.lock().unwrap();
                        let _ = s.sync_mods_to_profile();
                    });
                    self.status_message = "Syncing mods (Downloading)...".to_string();
                }
            });

            ui.add_space(10.0);

            // --- PROFILE MANAGEMENT SECTION ---
            ui.collapsing("Profile Management", |ui| {
                ui.horizontal(|ui| {
                    ui.label("New Profile Name:");
                    ui.text_edit_singleline(&mut self.new_profile_name);
                });

                if ui.button("Create Profile").clicked() {
                    let name = self.new_profile_name.trim().to_lowercase();
                    if !name.is_empty() {
                        if let Err(e) = state.make_new_profile(&name) {
                            self.status_message = format!("Error: {}", e);
                        } else {
                            self.status_message = format!("Created profile {}", name);
                        }
                    }
                }

                ui.separator();

                if ui.button("List Available Profiles").clicked() {
                    if let Ok(profiles) = state.list_available_profiles() {
                        self.available_profiles = profiles;
                    }
                }

                // Display list of profiles as buttons
                for profile in &self.available_profiles {
                    if ui.button(format!("Switch to {}", profile)).clicked() {
                        if let Err(e) = state.switch_current_profile(profile) {
                            self.status_message = format!("Error: {}", e);
                        } else {
                            self.status_message = format!("Switched to {}", profile);
                        }
                    }
                }
            });

            ui.add_space(20.0);
            ui.separator();
            
            // Status Bar
            ui.horizontal(|ui| {
                if self.is_loading {
                    ui.spinner();
                }
                ui.label(format!("Status: {}", self.status_message));
            });
        });

        // Tell egui to repaint frequently if we are loading
        if self.is_loading {
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 500.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Unruster Mod Manager",
        options,
        Box::new(|cc| Ok(Box::new(UnrusterApp::new(cc)))),
    )
}