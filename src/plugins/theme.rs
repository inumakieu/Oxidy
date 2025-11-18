use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crossterm::style::Color;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Theme {
    pub Background: Option<String>,
    pub Foreground: Option<String>,
    
    pub Comment: Option<String>,

    pub Namespace: Option<String>,
    pub Type: Option<String>,
    pub Class: Option<String>,
    pub Struct: Option<String>,
    pub Enum: Option<String>,
    pub Interface: Option<String>,
    pub TypeParameter: Option<String>,

    pub Variable: Option<String>,
    pub Parameter: Option<String>,
    pub Property: Option<String>,
    pub EnumMember: Option<String>,

    pub Function: Option<String>,
    pub Method: Option<String>,
    pub Macro: Option<String>,
    pub Event: Option<String>,

    pub Keyword: Option<String>,
    pub Modifier: Option<String>,
    pub Operator: Option<String>,

    pub String: Option<String>,
    pub Number: Option<String>,
    pub Regexp: Option<String>
}

impl Theme {
    pub fn default(&self) -> Self {
        Self {
            Background:      Some("#161617".to_string()),
            Foreground:      Some("#c9c7cd".to_string()),
            Comment:         Some("#8b8693".to_string()),

            Namespace:       Some("#ea83a5".to_string()),
            Type:            Some("#e6b99d".to_string()),
            Class:           Some("#e6b99d".to_string()),
            Struct:          Some("#e6b99d".to_string()),
            Enum:            Some("#e6b99d".to_string()),
            Interface:       Some("#e6b99d".to_string()),
            TypeParameter:   Some("#9f9ca6".to_string()),

            Variable:        Some("#c9c7cd".to_string()),
            Parameter:       Some("#b4b1ba".to_string()),
            Property:        Some("#d8b38b".to_string()),
            EnumMember:      Some("#d8b38b".to_string()),

            Function:        Some("#92a2d5".to_string()),
            Method:          Some("#92a2d5".to_string()),
            Macro:           Some("#ea83a5".to_string()),
            Event:           Some("#85b5ba".to_string()),

            Keyword:         Some("#aca1cf".to_string()),
            Modifier:        Some("#aca1cf".to_string()),
            Operator:        Some("#e3dcca".to_string()),

            String:          Some("#90b99f".to_string()),
            Number:          Some("#e29eca".to_string()),
            Regexp:          Some("#e29eca".to_string())
        }
    }

    pub fn to_map(&self) -> HashMap<String, Color> {
        let mut map = HashMap::new();

        macro_rules! add {
            ($field:ident) => {
                {
                    let key = {
                        let s = stringify!($field);
                        let mut chars = s.chars();
                        match chars.next() {
                            Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
                            None => String::new(),
                        }
                    };
                    
                    if let Some(hex) = &self.$field {
                        let hex = hex.trim_start_matches('#');
                        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or_default();
                        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or_default();
                        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or_default();

                        map.insert(key, Color::Rgb { r, g, b });
                    }
                }
            };
        }

        add!(Background);
        add!(Foreground);
        add!(Comment);

        add!(Namespace);
        add!(Type);
        add!(Class);
        add!(Struct);
        add!(Enum);
        add!(Interface);
        add!(TypeParameter);

        add!(Variable);
        add!(Parameter);
        add!(Property);
        add!(EnumMember);

        add!(Function);
        add!(Method);
        add!(Macro);
        add!(Event);

        add!(Keyword);
        add!(Modifier);
        add!(Operator);

        add!(String);
        add!(Number);
        add!(Regexp);

        map
    }

    pub fn merge(&self, base: &Theme) -> Theme {
        Theme {
            Background: self.Background.clone().or(base.Background.clone()),
            Foreground: self.Foreground.clone().or(base.Foreground.clone()),
            Comment:    self.Comment.clone().or(base.Comment.clone()),

            Namespace: self.Namespace.clone().or(base.Namespace.clone()),
            Type: self.Type.clone().or(base.Type.clone()),
            Class: self.Class.clone().or(base.Class.clone()),
            Struct: self.Struct.clone().or(base.Struct.clone()),
            Enum: self.Enum.clone().or(base.Enum.clone()),
            Interface: self.Interface.clone().or(base.Interface.clone()),
            TypeParameter: self.TypeParameter.clone().or(base.TypeParameter.clone()),

            Variable: self.Variable.clone().or(base.Variable.clone()),
            Parameter: self.Parameter.clone().or(base.Parameter.clone()),
            Property: self.Property.clone().or(base.Property.clone()),
            EnumMember: self.EnumMember.clone().or(base.EnumMember.clone()),

            Function: self.Function.clone().or(base.Function.clone()),
            Method: self.Method.clone().or(base.Method.clone()),
            Macro: self.Macro.clone().or(base.Macro.clone()),
            Event: self.Event.clone().or(base.Event.clone()),

            Keyword: self.Keyword.clone().or(base.Keyword.clone()),
            Modifier: self.Modifier.clone().or(base.Modifier.clone()),
            Operator: self.Operator.clone().or(base.Operator.clone()),

            String: self.String.clone().or(base.String.clone()),
            Number: self.Number.clone().or(base.Number.clone()),
            Regexp: self.Regexp.clone().or(base.Regexp.clone()),
        }
    }
}
