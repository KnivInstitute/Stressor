use eframe::egui;
use std::time::{Duration, Instant};
use log::info;
// use crate::app::SystemMonitorApp;

#[cfg(windows)]
fn is_running_as_admin() -> bool {
    use winapi::um::processthreadsapi::OpenProcessToken;
    use winapi::um::securitybaseapi::CheckTokenMembership;
    use winapi::um::winnt::{HANDLE, TOKEN_QUERY, PSID, SECURITY_NT_AUTHORITY, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS};
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use std::ptr::null_mut;
    unsafe {
        let mut is_admin = 0i32;
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut nt_authority = SECURITY_NT_AUTHORITY;
        let mut admin_group: PSID = null_mut();
        if winapi::um::securitybaseapi::AllocateAndInitializeSid(
            &mut nt_authority as *mut _ as *mut winapi::um::winnt::SID_IDENTIFIER_AUTHORITY,
            2,
            SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS,
            0, 0, 0, 0, 0, 0,
            &mut admin_group
        ) == 0 {
            return false;
        }
        let result = CheckTokenMembership(token, admin_group, &mut is_admin);
        winapi::um::securitybaseapi::FreeSid(admin_group);
        is_admin != 0 && result != 0
    }
}
#[cfg(not(windows))]
fn is_running_as_admin() -> bool { true }

pub struct OnLoadApp {
    start_time: Instant,
    progress: f32,
    pub done: bool,
    pub is_admin: bool,
    pub dev_mode: bool,
    space_held_since: Option<Instant>,
    space_press_times: Vec<Instant>,
    dev_mode_activated_time: Option<Instant>,
    hold_after_progress_until: Option<Instant>,
    space_detected_time: Option<Instant>,
}

impl Default for OnLoadApp {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            progress: 0.0,
            done: false,
            is_admin: is_running_as_admin(),
            dev_mode: false,
            space_held_since: None,
            space_press_times: Vec::new(),
            dev_mode_activated_time: None,
            hold_after_progress_until: None,
            space_detected_time: None,
        }
    }
}

impl eframe::App for OnLoadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let space_down = ctx.input(|i| i.key_down(egui::Key::Space));
        let space_pressed = ctx.input(|i| i.key_pressed(egui::Key::Space));
        // If space is pressed or held, record the first detection time
        if (space_down || space_pressed) && self.space_detected_time.is_none() {
            self.space_detected_time = Some(now);
            info!("[DEV] First space bar detected, starting 5s grace period");
        }
        if space_down {
            if self.space_held_since.is_none() {
                self.space_held_since = Some(now);
                info!("[DEV] Space bar pressed (down)");
            }
            if let Some(start) = self.space_held_since {
                if now.duration_since(start) > Duration::from_secs(1) && !self.dev_mode {
                    self.dev_mode = true;
                    self.dev_mode_activated_time = Some(now);
                    info!("[DEV] Dev mode activated via space bar (held)");
                    self.space_press_times.clear();
                }
            }
            if self.progress >= 1.0 {
                self.hold_after_progress_until = Some(now + Duration::from_secs(2));
            }
        } else {
            self.space_held_since = None;
        }
        // Track presses
        if space_pressed {
            self.space_press_times.push(now);
            self.space_press_times.retain(|&t| now.duration_since(t) <= Duration::from_secs(3));
            info!("[DEV] Space bar pressed (pressed), count in window: {}", self.space_press_times.len());
            if self.space_press_times.len() >= 3 && !self.dev_mode {
                self.dev_mode = true;
                self.dev_mode_activated_time = Some(now);
                info!("[DEV] Dev mode activated via space bar (pressed 3x)");
                self.space_press_times.clear();
            }
        }
        let elapsed = self.start_time.elapsed().as_secs_f32();
        // Simulate loading progress
        if self.progress < 1.0 {
            self.progress = (elapsed / 2.0).min(1.0); // 2 seconds to load
        }
        // Only finish if not in hold-after-progress window
        if self.progress >= 1.0 {
            // If space was detected, wait 5 seconds from first detection before finishing
            if let Some(space_time) = self.space_detected_time {
                if now.duration_since(space_time) < Duration::from_secs(5) {
                    // Still waiting for 5s grace period
                } else {
                    self.done = true;
                }
            } else {
                // No space detected, use the original hold_after_progress_until logic
                match self.hold_after_progress_until {
                    Some(t) if now < t => {
                        // Still waiting for grace period
                    }
                    _ => {
                        self.done = true;
                    }
                }
            }
        }
        // If done, return and let the state machine in main.rs handle the transition
        if self.done {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                if let Ok(image) = image::open("logo.png") {
                    let image_buffer = image.resize(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
                    let tex = ctx.load_texture(
                        "logo",
                        egui::ColorImage::from_rgba_unmultiplied([
                            image_buffer.width() as usize,
                            image_buffer.height() as usize
                        ],
                        image_buffer.as_flat_samples().as_slice()),
                        egui::TextureOptions::default(),
                    );
                    ui.image(&tex);
                } else {
                    ui.heading("System Monitor & Stress Tool");
                }
                ui.add_space(20.0);
                ui.label("Loading...");
                
                if self.dev_mode {
                    let now = Instant::now();
                    let flash = (elapsed * 4.0).sin().abs() > 0.5;
                    if let Some(activated) = self.dev_mode_activated_time {
                        if now.duration_since(activated) < Duration::from_secs(5) {
                            let bar = egui::ProgressBar::new(self.progress)
                                .show_percentage()
                                .fill(if flash { egui::Color32::YELLOW } else { egui::Color32::RED });
                            ui.add(bar);
                            ui.colored_label(egui::Color32::YELLOW, "DEV MODE ACTIVATED");
                        } else {
                            let bar = egui::ProgressBar::new(self.progress)
                                .show_percentage();
                            ui.add(bar);
                            ui.colored_label(egui::Color32::YELLOW, "DEV MODE ACTIVE");
                        }
                    } else {
                        let bar = egui::ProgressBar::new(self.progress)
                            .show_percentage();
                        ui.add(bar);
                        ui.colored_label(egui::Color32::YELLOW, "DEV MODE ACTIVE");
                    }
                } else {
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                }
                if !self.is_admin {
                    ui.colored_label(egui::Color32::YELLOW, "Warning: This app requires administrator privileges to write logs.");
                }
            });
        });
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
