mod app;
use crate::app::onload::OnLoadApp;
use crate::app::SystemMonitorApp;

enum AppState {
    Splash(OnLoadApp),
    Main(SystemMonitorApp),
}

struct RootApp {
    state: AppState,
}

impl eframe::App for RootApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        match &mut self.state {
            AppState::Splash(splash) => {
                splash.update(ctx, frame);
                if splash.done {
                    let dev_mode = splash.dev_mode;
                    self.state = AppState::Main(SystemMonitorApp::with_dev_mode(dev_mode));
                }
            }
            AppState::Main(main_app) => {
                main_app.update(ctx, frame);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Stressor",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(RootApp { state: AppState::Splash(OnLoadApp::default()) }))),
    )
}