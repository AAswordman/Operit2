use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct PackageManager {
    activatedPackages: BTreeSet<String>,
}

impl PackageManager {
    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        self.activatedPackages.insert(packageName.to_string())
    }

    pub fn isPackageActivated(&self, packageName: &str) -> bool {
        self.activatedPackages.contains(packageName)
    }
}
