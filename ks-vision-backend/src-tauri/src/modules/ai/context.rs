use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContextMetadata {
    pub source: String,
    pub length: usize,
}
