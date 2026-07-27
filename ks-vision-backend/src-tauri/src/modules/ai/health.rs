use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct HealthStatus {
    pub available: bool,
    pub url: String,
    pub message: String,
}
