use serde::Deserialize;

#[derive(Deserialize)]
pub struct CargoMetadata {
    pub workspace_members: Vec<String>,
    pub packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
pub struct CargoPackage {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<CargoDependency>,
}

#[derive(Deserialize)]
pub struct CargoDependency {
    pub name: String,
    pub kind: Option<String>,
}
