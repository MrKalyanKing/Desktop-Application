use std::sync::Mutex;
use crate::modules::config::settings::Settings;

#[allow(dead_code)]
pub struct AppState {
    pub settings: Mutex<Settings>,
}
