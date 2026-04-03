fn main() -> iced::Result {
    frostscan::init_logging();
    tracing::info!("starting FrostScan application");
    frostscan::app::run()
}
