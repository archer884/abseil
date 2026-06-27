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
    XdgData,
    XdgConfig,
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) enum Root {
    PlatformConfig(ProjectDirs),
    PlatformData(ProjectDirs),
    Xdg(PathBuf),
    Path(PathBuf),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum XdgKind {
    Data,
    Config,
}

impl XdgKind {
    pub(crate) fn env_var(self) -> &'static str {
        match self {
            XdgKind::Data => "XDG_DATA_HOME",
            XdgKind::Config => "XDG_CONFIG_HOME",
        }
    }

    pub(crate) fn default_subdir(self) -> &'static str {
        match self {
            XdgKind::Data => ".local/share",
            XdgKind::Config => ".config",
        }
    }
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
            Root::Xdg(path) => path,
            Root::Path(path) => path,
        }
    }
}

/// Computes the XDG-style path for an application.
///
/// Honors `xdg_home` if `Some`; otherwise falls back to `home/.local/share`
/// (for data) or `home/.config` (for config). Returns `None` if neither base
/// is available.
///
/// This is a pure function so the path logic can be tested without touching
/// the process environment.
pub(crate) fn xdg_path(
    kind: XdgKind,
    application: &str,
    xdg_home: Option<&Path>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    let base = match xdg_home {
        Some(p) => p.to_path_buf(),
        None => home?.join(kind.default_subdir()),
    };
    Some(base.join(application))
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path().display())
    }
}
