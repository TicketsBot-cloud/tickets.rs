use super::ComponentType;
use crate::guild::Emoji;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug)]
pub struct OverflowMenu {
    pub r#type: ComponentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub groups: Vec<OverflowMenuGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverflowMenuGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Box<str>>,
    pub components: Vec<MenuActionItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MenuActionItem {
    pub r#type: ComponentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<Box<str>>,
    pub label: Box<str>,
    pub style: MenuActionItemStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(default = "bool::default")]
    pub disabled: bool,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum MenuActionItemStyle {
    Default = 1,
    Danger = 2,
    Link = 3,
}

#[cfg(test)]
mod tests {
    use super::super::Component;
    use super::*;

    #[test]
    fn round_trips_in_action_row() {
        let cases = [
            r#"{"type":1,"components":[{"type":25,"groups":[{"label":"Ticket","components":[{"type":26,"custom_id":"close","label":"Close","style":2,"disabled":false}]}]}]}"#,
            r#"{"type":25,"id":3,"groups":[]}"#,
            r#"{"type":25,"groups":[{"components":[{"type":26,"label":"Docs","style":3,"url":"https://docs.tickets.bot","disabled":true}]}]}"#,
        ];

        for case in cases {
            let parsed: Component = serde_json::from_str(case).expect(case);
            let out = serde_json::to_value(&parsed).unwrap();
            let expected: serde_json::Value = serde_json::from_str(case).unwrap();
            assert_eq!(out, expected, "{case}");
        }
    }

    #[test]
    fn parses_as_section_accessory() {
        let json = r#"{"type":9,"components":[],"accessory":{"type":25,"groups":[]}}"#;
        match serde_json::from_str::<Component>(json).unwrap() {
            Component::Section(s) => {
                assert!(matches!(s.accessory, Some(Component::OverflowMenu(_))))
            }
            other => panic!("expected section, got {other:?}"),
        }
    }

    #[test]
    fn disabled_defaults_to_false() {
        let item: MenuActionItem =
            serde_json::from_str(r#"{"type":26,"label":"Claim","style":1}"#).unwrap();
        assert!(!item.disabled);
        assert!(matches!(item.style, MenuActionItemStyle::Default));
    }

    #[test]
    fn rejects_unknown_style() {
        assert!(serde_json::from_str::<MenuActionItem>(
            r#"{"type":26,"label":"x","style":4}"#
        )
        .is_err());
    }
}
