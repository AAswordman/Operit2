//! File access and mount resolution shared by Core and platform host consumers.
//! This crate depends only on platform contracts, never tools, providers or runtime assembly.
//! Mapping is private so consumers cannot assemble another conversion pipeline.
//! ```compile_fail
//! use operit_vfs::PathMapper::PathMapper;
//! ```
#![allow(non_snake_case)]

pub mod FileReference;
pub mod FileChanges;
pub mod MountRegistry;
mod PathMapper;
pub mod StorageManagement;
pub mod VisualFileSystem;

pub use FileReference::{FileReference as VfsFile, VfsPath};
