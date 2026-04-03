pub mod message;
pub mod notice;
pub mod state;
pub mod update;
pub mod view;

pub use state::FrostScanApp;

pub fn run() -> iced::Result {
    iced::application(FrostScanApp::new, FrostScanApp::update, FrostScanApp::view)
        .subscription(FrostScanApp::subscription)
        .theme(FrostScanApp::theme)
        .run()
}
