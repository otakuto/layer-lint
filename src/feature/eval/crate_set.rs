use std::collections::HashSet;

use crate::CrateName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSet(pub HashSet<CrateName>);
