//! Exercises the production daemon host modules without unrelated host capabilities.

#[cfg(windows)]
#[path = "../../windows/src/daemon.rs"]
mod daemon;
#[cfg(unix)]
#[path = "../../unix/daemon.rs"]
mod daemon;
