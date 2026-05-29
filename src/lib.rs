mod location;

use std::{fmt, fs, io};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::location::{Dir, Location};

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
    /// A serialization or deserialization error occurred.
    Serialization(stringify::Error),
}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::IO(e) => e,
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
    /// Returns `T::default()` if no persisted state exists.
    pub fn load<T>(&self) -> Result<T>
    where
        T: Default + for<'a> Deserialize<'a>,
    {
        let dir = self.location.path();
        let path = dir.join(self.filename());

        if path.exists() {
            let text = fs::read_to_string(path)?;
            return stringify::from_str(&text)
                .or_else(|_| stringify::from_str::<Abseil<T>>(&text).map(Abseil::into_inner))
                .map_err(Into::into);
        }

        let legacy_path = dir.join("persist.json");
        if legacy_path.exists() {
            let text = fs::read_to_string(legacy_path)?;
            return stringify::from_str(&text)
                .or_else(|_| stringify::from_str::<Abseil<T>>(&text).map(Abseil::into_inner))
                .map_err(Into::into);
        }

        Ok(Default::default())
    }

    /// Serializes and writes the given state to the storage file.
    ///
    /// Creates the storage directory if it does not exist.
    pub fn store(&self, state: impl Serialize) -> Result<()> {
        let dir = self.location.path();

        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }

        let path = dir.join(self.filename());
        let text = self.stringify(state)?;
        Ok(fs::write(path, text)?)
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
    dir: Dir,
}

impl ProviderBuilder {
    /// Resolves the storage location and returns a configured [`Provider`].
    ///
    /// Returns [`Error::AppData`] if the platform data directory cannot be determined.
    pub fn build(self) -> Result<Provider> {
        let directories = ProjectDirs::from(
            self.qualifier.as_deref().unwrap_or(""),
            self.organization.as_deref().unwrap_or(""),
            &self.application,
        )
        .ok_or(Error::AppData(self.application))?;

        Ok(Provider {
            location: Location::new(directories, self.dir),
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
    pub fn use_config_dir(self) -> Self {
        Self {
            dir: Dir::Config,
            ..self
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Abseil<T> {
    pub state: T,
}

impl<T> Abseil<T> {
    pub fn into_inner(self) -> T {
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
