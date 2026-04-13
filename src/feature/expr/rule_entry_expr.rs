use anyhow::anyhow;

use crate::PolicyKind;
use crate::feature::config::{YamlPolicyEntry, YamlRuleEntry};
use super::{CrateSetExpr, PolicyEntryExpr};

pub struct RuleEntryExpr {
    pub from: CrateSetExpr,
    pub internal: Vec<PolicyEntryExpr>,
    pub external: Vec<PolicyEntryExpr>,
}

impl TryFrom<YamlRuleEntry> for RuleEntryExpr {
    type Error = anyhow::Error;

    fn try_from(yaml: YamlRuleEntry) -> anyhow::Result<Self> {
        let from = CrateSetExpr::try_from(yaml.from)?;
        let internal = convert_policies(yaml.internal)?;
        let external = convert_policies(yaml.external)?;
        Ok(RuleEntryExpr { from, internal, external })
    }
}

fn convert_policies(entries: Vec<YamlPolicyEntry>) -> anyhow::Result<Vec<PolicyEntryExpr>> {
    fn convert_crate_sets(cs: Vec<crate::feature::config::YamlCrateSet>) -> anyhow::Result<Vec<CrateSetExpr>> {
        cs.into_iter().map(CrateSetExpr::try_from).collect()
    }

    let mut result = Vec::new();
    for entry in entries {
        match (entry.deny, entry.allow, entry.ignore) {
            (Some(cs), None, None) => result.push(PolicyEntryExpr { policy: PolicyKind::Deny, crate_sets: convert_crate_sets(cs)? }),
            (None, Some(cs), None) => result.push(PolicyEntryExpr { policy: PolicyKind::Allow, crate_sets: convert_crate_sets(cs)? }),
            (None, None, Some(cs)) => result.push(PolicyEntryExpr { policy: PolicyKind::Ignore, crate_sets: convert_crate_sets(cs)? }),
            (None, None, None) => return Err(anyhow!("policy entry must have exactly one of 'allow', 'deny', or 'ignore'")),
            _ => return Err(anyhow!("policy entry must have exactly one of 'allow', 'deny', or 'ignore', not multiple")),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::config::{YamlCrateSet, YamlPolicyEntry};

    fn yaml_crate(name: &str) -> YamlCrateSet {
        YamlCrateSet { crate_name: Some(crate::CrateName(name.to_string())), regex: None, layer: None, exclude: None }
    }

    #[test]
    fn single_allow() {
        let entries = vec![YamlPolicyEntry { allow: Some(vec![yaml_crate("serde")]), deny: None, ignore: None }];
        let result = convert_policies(entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].policy, PolicyKind::Allow);
    }

    #[test]
    fn single_deny() {
        let entries = vec![YamlPolicyEntry { deny: Some(vec![yaml_crate("diesel")]), allow: None, ignore: None }];
        let result = convert_policies(entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].policy, PolicyKind::Deny);
    }

    #[test]
    fn single_ignore() {
        let entries = vec![YamlPolicyEntry { ignore: Some(vec![yaml_crate("uuid")]), deny: None, allow: None }];
        let result = convert_policies(entries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].policy, PolicyKind::Ignore);
    }

    #[test]
    fn empty_entry_errors() {
        let entries = vec![YamlPolicyEntry { allow: None, deny: None, ignore: None }];
        assert!(convert_policies(entries).is_err());
    }

    #[test]
    fn multiple_policies_in_one_entry_errors() {
        let entries = vec![YamlPolicyEntry {
            allow: Some(vec![yaml_crate("serde")]),
            deny: Some(vec![yaml_crate("diesel")]),
            ignore: None,
        }];
        assert!(convert_policies(entries).is_err());
    }

    #[test]
    fn multiple_entries_preserve_order() {
        let entries = vec![
            YamlPolicyEntry { deny: Some(vec![yaml_crate("diesel")]), allow: None, ignore: None },
            YamlPolicyEntry { allow: Some(vec![yaml_crate("serde")]), deny: None, ignore: None },
        ];
        let result = convert_policies(entries).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].policy, PolicyKind::Deny);
        assert_eq!(result[1].policy, PolicyKind::Allow);
    }
}
