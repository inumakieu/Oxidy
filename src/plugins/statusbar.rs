use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum StatusComponent {
    Text(String),           // static text
    Field(String),          // dynamic field: "filename", "mode", "git_branch"
    Eval(String),           // Rhai expression -> string
    Spacer,
    Group(Vec<StatusComponent>),
    Color {
        fg: Option<String>,
        bg: Option<String>,
        content: Box<StatusComponent>
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StatusBarConfig {
    pub components: Vec<StatusComponent>,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            components: vec![
                StatusComponent::Group(vec![
                    StatusComponent::Text("Oxidy".into())
                ]),
                StatusComponent::Group(vec![
                    StatusComponent::Field("filename".into())
                ]),
                StatusComponent::Spacer,
                StatusComponent::Group(vec![
                    StatusComponent::Eval("format('{}:{} {}', line, total_lines, mode)".into())
                ])
            ]
        }
    }
}
