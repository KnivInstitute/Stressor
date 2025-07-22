mod app;

// fn main() {
//     app::stress_test::print_cpu_temperatures();
// }
fn main() -> eframe::Result<()> {
    app::run_app()
}