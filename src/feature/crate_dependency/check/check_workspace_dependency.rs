use crate::LintError;
use super::internal::check_all_members_included;
use super::super::WorkspaceDependency;

pub fn check_workspace_dependency(workspace: &WorkspaceDependency) -> Vec<LintError> {
    let mut errors = Vec::new();
    errors.extend(check_all_members_included(workspace));
    errors
}
