use std::collections::{HashMap, HashSet};

use crate::CrateName;
use crate::feature::eval::CrateSet;
use crate::feature::crate_dependency::{CrateDependency, WorkspaceDependency};
use super::CargoMetadata;

pub fn metadata_to_dependencies(metadata: CargoMetadata) -> WorkspaceDependency {
    let workspace_member_ids: HashSet<String> = metadata.workspace_members.into_iter().collect();

    let mut members: HashMap<CrateName, CrateDependency> = HashMap::new();
    let mut member_names: HashSet<CrateName> = HashSet::new();

    for package in &metadata.packages {
        if workspace_member_ids.contains(&package.id) {
            member_names.insert(CrateName(package.name.clone()));
        }
    }

    let mut external_set: HashSet<CrateName> = HashSet::new();

    for package in metadata.packages {
        if !workspace_member_ids.contains(&package.id) {
            continue;
        }

        let deps: HashSet<CrateName> = package
            .dependencies
            .into_iter()
            .filter(|d| d.kind.is_none())
            .map(|d| CrateName(d.name))
            .collect();

        for dep in &deps {
            if !member_names.contains(dep) {
                external_set.insert(dep.clone());
            }
        }

        let name = CrateName(package.name);
        let crate_dep = CrateDependency { from: name.clone(), to: CrateSet(deps) };
        members.insert(name, crate_dep);
    }

    let mut external: Vec<CrateName> = external_set.into_iter().collect();
    external.sort_by(|a, b| a.0.cmp(&b.0));

    WorkspaceDependency { members, external }
}
