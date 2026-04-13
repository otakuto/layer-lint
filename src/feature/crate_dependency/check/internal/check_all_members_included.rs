use crate::LintError;
use super::super::super::WorkspaceDependency;

pub fn check_all_members_included(_workspace: &WorkspaceDependency) -> Vec<LintError> {
    Vec::new()
}
