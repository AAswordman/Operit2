//! Exercise production update modules independently of unrelated utility test fixtures.
#![allow(non_snake_case, dead_code)]
pub use operit_util::{AppLogger, RuntimeStorageLayout};
#[path = "../src/GithubReleaseUtil.rs"]
mod GithubReleaseUtil;
#[path = "../src/UpdateHttp.rs"]
mod UpdateHttp;
#[path = "../src/UpdateSources.rs"]
mod UpdateSources;
