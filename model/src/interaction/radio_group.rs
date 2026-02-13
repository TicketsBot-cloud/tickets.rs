use super::ComponentType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RadioGroup {
    pub r#type: ComponentType,
    pub custom_id: Box<str>,
    pub options: Vec<RadioGroupOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RadioGroupOption {
    pub value: Box<str>,
    pub label: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Box<str>>,
    #[serde(default)]
    pub default: bool,
}
