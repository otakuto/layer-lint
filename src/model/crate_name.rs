#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize)]
pub struct CrateName(pub String);

impl std::fmt::Display for CrateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
