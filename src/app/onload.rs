use eframe::egui;
use std::time::{Duration, Instant};

pub struct OnLoadApp {
    start_time: Instant,
    progress: f32,
    pub done: bool,
}

impl Default for OnLoadApp {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            progress: 0.0,
            done: false,
        }
    }
}

impl eframe::App for OnLoadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        // Simulate loading progress
        if self.progress < 1.0 {
            self.progress = (elapsed / 2.0).min(1.0); // 2 seconds to load
        } else {
            self.done = true;
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
                ui.add(egui::ProgressBar::new(self.progress).show_percentage());
            });
        });
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
