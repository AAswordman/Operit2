use std::path::PathBuf;

#[allow(non_snake_case)]
pub fn repoFixtureToolPkgPath(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("toolpkg")
        .join(name)
}
