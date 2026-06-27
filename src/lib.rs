#![doc = include_str!("../README.md")]

mod location;

use std::{fmt, fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::location::{Location, Root, RootLazy, XdgKind, xdg_path};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(feature = "json")]
const DEFAULT_FILENAME: &str = "storage.json";

#[cfg(all(feature = "toml", not(feature = "json")))]
const DEFAULT_FILENAME: &str = "storage.toml";

/// Errors that can occur when loading or storing application state.
#[derive(Debug)]
pub enum Error {
    /// The platform-specific application data directory could not be determined.
    AppData(String),
    /// An I/O error occurred while reading or writing the storage file.
    IO(io::Error),
    /// No persisted state was found for the application.
    NotFound,
    /// A serialization or deserialization error occurred.
    Serialization(stringify::Error),
}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::IO(e) => e,
            Error::NotFound => io::Error::new(io::ErrorKind::NotFound, "no persisted state found"),
            Error::Serialization(e) => io::Error::new(io::ErrorKind::InvalidData, e),
            e => io::Error::other(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IO(value)
    }
}

impl From<stringify::Error> for Error {
    fn from(value: stringify::Error) -> Self {
        Error::Serialization(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AppData(name) => write!(f, "unable to open storage for {name}"),
            Error::IO(e) => e.fmt(f),
            Error::NotFound => write!(f, "no persisted state found"),
            Error::Serialization(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// Persists application state to a platform-specific data directory.
///
/// Create a `Provider` via [`Provider::builder`]. Use [`store`](Provider::store) to write
/// state and [`load`](Provider::load) to read it back.
#[derive(Debug, Clone)]
pub struct Provider {
    location: Location,
    filename: Option<String>,
    pretty: bool,
}

impl Provider {
    /// Returns a [`ProviderBuilder`] for the given application name.
    pub fn builder(application: impl Into<String>) -> ProviderBuilder {
        ProviderBuilder {
            application: application.into(),
            ..Default::default()
        }
    }

    /// Loads persisted state from storage.
    ///
    /// Attempts to deserialize directly as `T` first. If that fails, falls back to
    /// deserializing as a legacy `Abseil<T>` wrapper and extracts the inner state.
    /// Also checks for a legacy `persist.json` file if the primary file is missing.
    /// Returns [`Error::NotFound`] if no persisted state exists.
    pub fn load<T>(&self) -> Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let dir = self.location.path();
        let path = dir.join(self.filename());

        match self.try_load_file::<T>(&path) {
            Ok(state) => Ok(state),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let legacy_path = dir.join("persist.json");
                match self.try_load_file::<T>(&legacy_path) {
                    Ok(state) => Ok(state),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Error::NotFound),
                    Err(e) => Err(e.into()),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Loads persisted state from storage, returning `T::default()` if none exists.
    ///
    /// Behaves like [`load`](Provider::load) but returns a default value instead
    /// of an error when no persisted state is found.
    pub fn load_or_default<T>(&self) -> Result<T>
    where
        T: Default + for<'a> Deserialize<'a>,
    {
        match self.load() {
            Ok(state) => Ok(state),
            Err(Error::NotFound) => Ok(Default::default()),
            Err(e) => Err(e),
        }
    }

    fn try_load_file<T: for<'a> Deserialize<'a>>(&self, path: &std::path::Path) -> io::Result<T> {
        let text = fs::read_to_string(path)?;
        stringify::from_str(&text)
            .or_else(|_| stringify::from_str::<Abseil<T>>(&text).map(Abseil::into_inner))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Serializes and writes the given state to the storage file.
    ///
    /// Creates the storage directory if it does not exist. Writes are atomic:
    /// a temporary file is written and renamed to prevent corruption on crash.
    pub fn store(&self, state: impl Serialize) -> Result<()> {
        let dir = self.location.path();
        fs::create_dir_all(dir)?;

        let path = dir.join(self.filename());
        let text = self.stringify(state)?;

        let mut tmp = NamedTempFile::new_in(dir)?;
        io::Write::write_all(&mut tmp, text.as_bytes())?;
        tmp.persist(path).map_err(|e| io::Error::other(e.error))?;

        Ok(())
    }

    /// Returns a reference to the resolved storage [`Location`].
    pub fn location(&self) -> &Location {
        &self.location
    }

    fn filename(&self) -> &str {
        self.filename.as_deref().unwrap_or(DEFAULT_FILENAME)
    }

    fn stringify(&self, state: impl Serialize) -> stringify::Result<String> {
        if self.pretty {
            stringify::to_string_pretty(&state)
        } else {
            stringify::to_string(&state)
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.location)
    }
}

/// Builds a [`Provider`] with custom configuration.
///
/// Obtain a builder via [`Provider::builder`]. Chain configuration methods,
/// then call [`build`](ProviderBuilder::build) to produce a `Provider`.
#[derive(Debug, Default)]
pub struct ProviderBuilder {
    qualifier: Option<String>,
    organization: Option<String>,
    application: String,
    pretty: bool,
    filename: Option<String>,
    root: RootLazy,
}

impl ProviderBuilder {
    /// Resolves the storage location and returns a configured [`Provider`].
    ///
    /// Returns [`Error::AppData`] if the platform data directory cannot be determined
    /// and no explicit path was set via [`with_path`](ProviderBuilder::with_path).
    pub fn build(self) -> Result<Provider> {
        let root = match self.root {
            RootLazy::Path(path) => Root::Path(path),
            RootLazy::PlatformData | RootLazy::PlatformConfig => {
                let directories = ProjectDirs::from(
                    self.qualifier.as_deref().unwrap_or(""),
                    self.organization.as_deref().unwrap_or(""),
                    &self.application,
                )
                .ok_or(Error::AppData(self.application.clone()))?;
                match self.root {
                    RootLazy::PlatformConfig => Root::PlatformConfig(directories),
                    _ => Root::PlatformData(directories),
                }
            }
            RootLazy::XdgData | RootLazy::XdgConfig => {
                let kind = match self.root {
                    RootLazy::XdgConfig => XdgKind::Config,
                    _ => XdgKind::Data,
                };
                if cfg!(target_os = "macos") {
                    let xdg_home = std::env::var_os(kind.env_var())
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from);
                    let home = std::env::var_os("HOME")
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from);
                    let path = xdg_path(
                        kind,
                        &self.application,
                        xdg_home.as_deref(),
                        home.as_deref(),
                    )
                    .ok_or(Error::AppData(self.application.clone()))?;
                    Root::Xdg(path)
                } else {
                    let directories = ProjectDirs::from(
                        self.qualifier.as_deref().unwrap_or(""),
                        self.organization.as_deref().unwrap_or(""),
                        &self.application,
                    )
                    .ok_or(Error::AppData(self.application.clone()))?;
                    let path = match kind {
                        XdgKind::Data => directories.data_dir().to_path_buf(),
                        XdgKind::Config => directories.config_dir().to_path_buf(),
                    };
                    Root::Xdg(path)
                }
            }
        };

        Ok(Provider {
            location: Location::new(root),
            pretty: self.pretty,
            filename: self.filename,
        })
    }

    /// Sets the qualifier component of the reverse-domain application identifier.
    pub fn with_qualifier(self, qualifier: impl Into<String>) -> Self {
        Self {
            qualifier: Some(qualifier.into()),
            ..self
        }
    }

    /// Sets the organization component of the reverse-domain application identifier.
    pub fn with_organization(self, organization: impl Into<String>) -> Self {
        Self {
            organization: Some(organization.into()),
            ..self
        }
    }

    /// Format output when storing.
    pub fn pretty(self) -> Self {
        Self {
            pretty: true,
            ..self
        }
    }

    /// Set a custom filename for the storage file.
    pub fn with_filename(self, filename: impl Into<String>) -> Self {
        Self {
            filename: Some(filename.into()),
            ..self
        }
    }

    /// Store configuration in the config directory instead of the data directory.
    ///
    /// Composes with [`use_xdg_layout`](Self::use_xdg_layout): the config/data
    /// choice is preserved when switching between platform and XDG resolution.
    pub fn use_config_dir(self) -> Self {
        let root = match self.root {
            RootLazy::PlatformData | RootLazy::XdgData => RootLazy::PlatformConfig,
            RootLazy::PlatformConfig | RootLazy::XdgConfig => self.root,
            RootLazy::Path(_) => self.root,
        };
        Self { root, ..self }
    }

    /// Use an XDG-style directory layout on macOS instead of `~/Library/Application Support`.
    ///
    /// On macOS this resolves the data directory to
    /// `$XDG_DATA_HOME/<app>` (defaulting to `~/.local/share/<app>`) and the
    /// config directory to `$XDG_CONFIG_HOME/<app>` (defaulting to
    /// `~/.config/<app>`), matching the [XDG Base Directory Specification].
    /// The `XDG_DATA_HOME` and `XDG_CONFIG_HOME` environment variables are
    /// respected when set.
    ///
    /// On non-macOS platforms this is a no-op: the resolved path is identical
    /// to the platform default (`ProjectDirs::data_dir()` /
    /// `ProjectDirs::config_dir()`). Use [`with_path`](Self::with_path) to
    /// override the location on those platforms.
    ///
    /// Composes with [`use_config_dir`](Self::use_config_dir) to select the
    /// config directory under the XDG layout. If [`with_path`](Self::with_path)
    /// is also set, `with_path` wins.
    ///
    /// [XDG Base Directory Specification]: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
    pub fn use_xdg_layout(self) -> Self {
        let root = match self.root {
            RootLazy::PlatformData => RootLazy::XdgData,
            RootLazy::PlatformConfig => RootLazy::XdgConfig,
            other => other,
        };
        Self { root, ..self }
    }

    /// Use an explicit storage path instead of the platform-specific data directory.
    ///
    /// When set, this overrides the platform directory resolution entirely,
    /// regardless of any qualifier, organization, [`use_config_dir`], or
    /// [`use_xdg_layout`] setting.
    pub fn with_path(self, path: impl Into<PathBuf>) -> Self {
        Self {
            root: RootLazy::Path(path.into()),
            ..self
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Abseil<T> {
    state: T,
}

impl<T> Abseil<T> {
    fn into_inner(self) -> T {
        self.state
    }
}

#[cfg(feature = "json")]
mod stringify {
    use serde::{Deserialize, Serialize};

    pub type Result<T> = serde_json::Result<T>;

    pub type Error = serde_json::Error;

    pub fn to_string(value: &impl Serialize) -> Result<String> {
        serde_json::to_string(value)
    }

    pub fn to_string_pretty(value: &impl Serialize) -> Result<String> {
        serde_json::to_string_pretty(value)
    }

    pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T> {
        serde_json::from_str(s)
    }
}

#[cfg(all(feature = "toml", not(feature = "json")))]
mod stringify {
    use core::fmt;

    use serde::{Serialize, de::DeserializeOwned};

    pub type Result<T, E = Error> = std::result::Result<T, E>;

    #[derive(Debug)]
    pub enum Error {
        Serialization(toml::ser::Error),
        Deserialization(toml::de::Error),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Serialization(e) => e.fmt(f),
                Error::Deserialization(e) => e.fmt(f),
            }
        }
    }

    impl std::error::Error for Error {}

    pub fn to_string(value: &impl Serialize) -> Result<String> {
        toml::to_string(value).map_err(Error::Serialization)
    }

    pub fn to_string_pretty(value: &impl Serialize) -> Result<String> {
        toml::to_string_pretty(value).map_err(Error::Serialization)
    }

    pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T> {
        toml::from_str(s).map_err(Error::Deserialization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    struct AppState {
        name: String,
        count: u32,
    }

    fn test_provider(dir: &std::path::Path) -> Provider {
        Provider::builder("test-app")
            .with_path(dir)
            .build()
            .unwrap()
    }

    #[test]
    fn store_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        let state = AppState {
            name: "hello".into(),
            count: 42,
        };
        provider.store(&state).unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn load_errors_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        let err = provider.load::<AppState>().unwrap_err();
        assert!(matches!(err, Error::NotFound));
    }

    #[test]
    fn load_or_default_returns_default_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        let loaded: AppState = provider.load_or_default().unwrap();
        assert_eq!(loaded, AppState::default());
    }

    #[test]
    fn load_wrapped_payload() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());
        let path = dir.path().join(DEFAULT_FILENAME);

        fs::write(&path, r#"{"state": {"name": "wrapped", "count": 7}}"#).unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(
            loaded,
            AppState {
                name: "wrapped".into(),
                count: 7,
            }
        );
    }

    #[test]
    fn load_wrapped_payload_with_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());
        let path = dir.path().join(DEFAULT_FILENAME);

        fs::write(
            &path,
            r#"{"timestamp": "2024-01-01T00:00:00Z", "state": {"name": "legacy", "count": 3}}"#,
        )
        .unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(
            loaded,
            AppState {
                name: "legacy".into(),
                count: 3,
            }
        );
    }

    #[test]
    fn load_bare_payload() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());
        let path = dir.path().join(DEFAULT_FILENAME);

        fs::write(&path, r#"{"name": "bare", "count": 5}"#).unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(
            loaded,
            AppState {
                name: "bare".into(),
                count: 5,
            }
        );
    }

    #[test]
    fn load_falls_back_to_legacy_persist_json() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        fs::write(
            dir.path().join("persist.json"),
            r#"{"name": "legacy", "count": 1}"#,
        )
        .unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(
            loaded,
            AppState {
                name: "legacy".into(),
                count: 1,
            }
        );
    }

    #[test]
    fn load_prefers_primary_over_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());
        let path = dir.path().join(DEFAULT_FILENAME);

        fs::write(&path, r#"{"name": "primary", "count": 10}"#).unwrap();
        fs::write(
            dir.path().join("persist.json"),
            r#"{"name": "legacy", "count": 1}"#,
        )
        .unwrap();

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(
            loaded,
            AppState {
                name: "primary".into(),
                count: 10,
            }
        );
    }

    #[test]
    fn custom_filename() {
        let dir = tempfile::tempdir().unwrap();
        let provider = Provider::builder("test-app")
            .with_path(dir.path())
            .with_filename("custom.json")
            .build()
            .unwrap();

        let state = AppState {
            name: "custom".into(),
            count: 99,
        };
        provider.store(&state).unwrap();

        assert!(dir.path().join("custom.json").exists());

        let loaded: AppState = provider.load().unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn store_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let provider = test_provider(&nested);

        provider.store(AppState::default()).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn location_returns_path() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        assert_eq!(provider.location().path(), dir.path());
    }

    #[test]
    fn error_display() {
        let err = Error::AppData("my-app".into());
        assert_eq!(err.to_string(), "unable to open storage for my-app");

        let err = Error::NotFound;
        assert_eq!(err.to_string(), "no persisted state found");

        let err = Error::IO(io::Error::new(io::ErrorKind::BrokenPipe, "pipe"));
        assert!(err.to_string().contains("pipe"));

        let err = Error::Serialization(stringify::from_str::<AppState>("not json").unwrap_err());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn error_into_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "gone");
        let err: Error = io_err.into();
        let converted: io::Error = err.into();
        assert_eq!(converted.kind(), io::ErrorKind::NotFound);

        let err = Error::NotFound;
        let io_err: io::Error = err.into();
        assert_eq!(io_err.kind(), io::ErrorKind::NotFound);

        let err = Error::Serialization(stringify::from_str::<AppState>("bad").unwrap_err());
        let io_err: io::Error = err.into();
        assert_eq!(io_err.kind(), io::ErrorKind::InvalidData);

        let err = Error::AppData("test".into());
        let io_err: io::Error = err.into();
        assert_eq!(io_err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn provider_display_shows_location() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(dir.path());

        assert_eq!(provider.to_string(), dir.path().display().to_string());
    }

    #[test]
    fn xdg_path_data_default() {
        let home = std::path::Path::new("/home/user");
        let p = xdg_path(XdgKind::Data, "my-app", None, Some(home)).unwrap();
        assert_eq!(
            p,
            std::path::PathBuf::from("/home/user/.local/share/my-app")
        );
    }

    #[test]
    fn xdg_path_config_default() {
        let home = std::path::Path::new("/home/user");
        let p = xdg_path(XdgKind::Config, "my-app", None, Some(home)).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/home/user/.config/my-app"));
    }

    #[test]
    fn xdg_path_honors_xdg_data_home() {
        let xdg = std::path::Path::new("/srv/data");
        let p = xdg_path(XdgKind::Data, "my-app", Some(xdg), None).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/srv/data/my-app"));
    }

    #[test]
    fn xdg_path_honors_xdg_config_home() {
        let xdg = std::path::Path::new("/srv/cfg");
        let p = xdg_path(XdgKind::Config, "my-app", Some(xdg), None).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/srv/cfg/my-app"));
    }

    #[test]
    fn xdg_path_prefers_xdg_home_over_default() {
        let xdg = std::path::Path::new("/srv/data");
        let home = std::path::Path::new("/home/user");
        let p = xdg_path(XdgKind::Data, "my-app", Some(xdg), Some(home)).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/srv/data/my-app"));
    }

    #[test]
    fn xdg_path_returns_none_without_base() {
        assert!(xdg_path(XdgKind::Data, "my-app", None, None).is_none());
        assert!(xdg_path(XdgKind::Config, "my-app", None, None).is_none());
    }

    #[test]
    fn use_xdg_layout_composes_with_use_config_dir() {
        let p1 = Provider::builder("my-app")
            .use_xdg_layout()
            .use_config_dir()
            .with_path("/tmp")
            .build()
            .unwrap();
        let p2 = Provider::builder("my-app")
            .use_config_dir()
            .use_xdg_layout()
            .with_path("/tmp")
            .build()
            .unwrap();
        assert_eq!(p1.location().path(), p2.location().path());
    }

    #[test]
    fn use_xdg_layout_is_idempotent() {
        let p1 = Provider::builder("my-app")
            .use_xdg_layout()
            .with_path("/tmp")
            .build()
            .unwrap();
        let p2 = Provider::builder("my-app")
            .use_xdg_layout()
            .use_xdg_layout()
            .with_path("/tmp")
            .build()
            .unwrap();
        assert_eq!(p1.location().path(), p2.location().path());
    }

    #[test]
    fn with_path_overrides_xdg_layout() {
        let dir = tempfile::tempdir().unwrap();
        let provider = Provider::builder("my-app")
            .use_xdg_layout()
            .with_path(dir.path())
            .build()
            .unwrap();
        assert_eq!(provider.location().path(), dir.path());
    }
}
