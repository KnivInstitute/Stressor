mod app;
use crate::app::onload::OnLoadApp;
use crate::app::SystemMonitorApp;
use eframe::egui;

enum AppState {
    Splash(OnLoadApp),
    Main(SystemMonitorApp),
}

struct WrapperApp {
    state: AppState,
    switched: bool,
}

impl WrapperApp {
    fn new() -> Self {
        Self {
            state: AppState::Splash(OnLoadApp::default()),
            switched: false,
        }
    }
}

impl eframe::App for WrapperApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match &mut self.state {
            AppState::Splash(splash) => {
                splash.update(ctx, frame);
                if splash.done {
                    self.state = AppState::Main(SystemMonitorApp::default());
                    self.switched = true;
                }
            }
            AppState::Main(main_app) => {
                // Window resizing not supported in this eframe version
                main_app.update(ctx, frame);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    app::run_app_with(Box::new(WrapperApp::new()), true)
}