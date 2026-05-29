use core::fmt;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// The resolved storage directory for an application.
///
/// Wraps platform-specific directory resolution from the `directories` crate,
/// or holds an explicit path override set via [`ProviderBuilder::with_path`].
#[derive(Debug, Clone)]
pub struct Location {
    root: Root,
}

#[derive(Debug, Default, Clone)]
pub(crate) enum RootLazy {
    #[default]
    PlatformData,
    PlatformConfig,
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) enum Root {
    PlatformConfig(ProjectDirs),
    PlatformData(ProjectDirs),
    Path(PathBuf),
}

impl Location {
    pub(crate) fn new(root: Root) -> Self {
        Self { root }
    }

    /// Returns the path to the resolved storage directory.
    pub fn path(&self) -> &Path {
        match &self.root {
            Root::PlatformConfig(directories) => directories.config_dir(),
            Root::PlatformData(directories) => directories.data_dir(),
            Root::Path(path) => path,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path().display())
    }
}
