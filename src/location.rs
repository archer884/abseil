use core::fmt;
use std::path::Path;

use directories::ProjectDirs;

#[derive(Debug, Default, Copy, Clone)]
pub(crate) enum Dir {
    Config,
    #[default]
    Data,
}

/// The resolved storage directory for an application.
///
/// Wraps platform-specific directory resolution from the `directories` crate.
#[derive(Debug, Clone)]
pub struct Location {
    dir_option: Dir,
    directories: ProjectDirs,
}

impl Location {
    pub(crate) fn new(directories: ProjectDirs, dir_option: Dir) -> Self {
        Self {
            directories,
            dir_option,
        }
    }

    /// Returns the path to the resolved storage directory.
    pub fn path(&self) -> &Path {
        match self.dir_option {
            Dir::Config => self.directories.config_dir(),
            Dir::Data => self.directories.data_dir(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path().display())
    }
}
