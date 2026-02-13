use super::{
    ActionRow, Button, CheckboxGroup, Container, File, InputText, Label, MediaGallery,
    RadioGroup, Section, SelectMenu, Separator, TextDisplay, Thumbnail,
};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Component {
    ActionRow(Box<ActionRow>),
    Button(Box<Button>),
    SelectMenu(Box<SelectMenu>),
    InputText(Box<InputText>),
    Section(Box<Section>),           // 9
    TextDisplay(Box<TextDisplay>),   // 10
    Thumbnail(Box<Thumbnail>),       // 11
    MediaGallery(Box<MediaGallery>), // 12
    File(Box<File>),                 // 13
    Separator(Box<Separator>),       // 14
    // 15 & 16 are not used
    Container(Box<Container>),       // 17
    Label(Box<Label>),               // 18
    // 19 & 20 are not used
    RadioGroup(Box<RadioGroup>),       // 21
    CheckboxGroup(Box<CheckboxGroup>), // 22
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    SelectMenu = 3,
    InputText = 4,
    UserSelect = 5,
    RoleSelect = 6,
    MentionableSelect = 7,
    ChannelSelect = 8,
    Section = 9,
    TextDisplay = 10,
    Thumbnail = 11,
    MediaGallery = 12,
    File = 13,
    Separator = 14,
    // 15 & 16 are not used
    Container = 17,
    Label = 18,
    // 19 & 20 are not used
    RadioGroup = 21,
    CheckboxGroup = 22,
}

impl TryFrom<u64> for ComponentType {
    type Error = Box<str>;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::ActionRow,
            2 => Self::Button,
            3 => Self::SelectMenu,
            4 => Self::InputText,
            5 => Self::UserSelect,
            6 => Self::RoleSelect,
            7 => Self::MentionableSelect,
            8 => Self::ChannelSelect,
            9 => Self::Section,
            10 => Self::TextDisplay,
            11 => Self::Thumbnail,
            12 => Self::MediaGallery,
            13 => Self::File,
            14 => Self::Separator,
            // 15 & 16 are not used
            17 => Self::Container,
            18 => Self::Label,
            // 19 & 20 are not used
            21 => Self::RadioGroup,
            22 => Self::CheckboxGroup,
            _ => return Err(format!("invalid component type \"{}\"", value).into_boxed_str()),
        })
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;

        let component_type = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or_else(|| Box::from("component type was not an integer"))
            .and_then(ComponentType::try_from)
            .map_err(D::Error::custom)?;

        let component = match component_type {
            ComponentType::ActionRow => serde_json::from_value(value).map(Component::ActionRow),
            ComponentType::Button => serde_json::from_value(value).map(Component::Button),
            ComponentType::Section => serde_json::from_value(value).map(Component::Section),
            ComponentType::TextDisplay => serde_json::from_value(value).map(Component::TextDisplay),
            ComponentType::Thumbnail => serde_json::from_value(value).map(Component::Thumbnail),
            ComponentType::MediaGallery => {
                serde_json::from_value(value).map(Component::MediaGallery)
            }
            ComponentType::File => serde_json::from_value(value).map(Component::File),
            ComponentType::Separator => serde_json::from_value(value).map(Component::Separator),
            ComponentType::Container => serde_json::from_value(value).map(Component::Container),
            ComponentType::Label => serde_json::from_value(value).map(Component::Label),
            ComponentType::SelectMenu
            | ComponentType::UserSelect
            | ComponentType::RoleSelect
            | ComponentType::MentionableSelect
            | ComponentType::ChannelSelect => {
                serde_json::from_value(value).map(Component::SelectMenu)
            }
            ComponentType::InputText => serde_json::from_value(value).map(Component::InputText),
            ComponentType::RadioGroup => {
                serde_json::from_value(value).map(Component::RadioGroup)
            }
            ComponentType::CheckboxGroup => {
                serde_json::from_value(value).map(Component::CheckboxGroup)
            }
        }
        .map_err(D::Error::custom)?;

        Ok(component)
    }
}
