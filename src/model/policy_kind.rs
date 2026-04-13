#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyKind {
    Allow,
    Deny,
    Ignore,
}

impl PolicyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyKind::Allow => "allow",
            PolicyKind::Deny => "deny",
            PolicyKind::Ignore => "ignore",
        }
    }
}
