use crate::CrateName;
use crate::feature::eval::CrateSet;

pub struct CrateDependency {
    pub from: CrateName,
    pub to: CrateSet,
}
