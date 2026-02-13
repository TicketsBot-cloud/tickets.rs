use super::ComponentType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckboxGroup {
    pub r#type: ComponentType,
    pub custom_id: Box<str>,
    pub options: Vec<CheckboxGroupOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckboxGroupOption {
    pub value: Box<str>,
    pub label: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Box<str>>,
    #[serde(default)]
    pub default: bool,
}
