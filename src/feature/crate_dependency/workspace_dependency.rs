use std::collections::{HashMap, HashSet};

use crate::CrateName;
use crate::feature::eval::CrateSet;
use super::CrateDependency;

pub struct WorkspaceDependency {
    pub members: HashMap<CrateName, CrateDependency>,
    pub external: Vec<CrateName>,
}

impl WorkspaceDependency {
    pub fn as_dep_map(&self) -> HashMap<CrateName, CrateSet> {
        self.members.iter().map(|(name, dep)| {
            (name.clone(), dep.to.clone())
        }).collect()
    }

    pub fn all_crates(&self) -> CrateSet {
        let mut all: HashSet<CrateName> = self.members.keys().cloned().collect();
        all.extend(self.external.iter().cloned());
        CrateSet(all)
    }
}
