mod location;

use std::{fmt, fs, io};

use directories::ProjectDirs;
use jiff::Zoned;
use location::{Dir, Location};
use serde::{Deserialize, Serialize};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(feature = "json")]
const DEFAULT_FILENAME: &str = "storage.json";

#[cfg(all(feature = "toml", not(feature = "json")))]
const DEFAULT_FILENAME: &str = "storage.toml";

#[derive(Debug)]
pub enum Error {
    AppData(Provider),
    IO(io::Error),
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
            Error::AppData(persist) => write!(f, "unable to open storage for {persist}"),
            Error::IO(e) => e.fmt(f),
            Error::Serialization(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Default, Clone)]
pub struct Provider {
    qualifier: Option<String>,
    organization: Option<String>,
    application: String,
    pretty: bool,
    filename: Option<String>,
    dir: Dir,
}

impl Provider {
    pub fn new(application: impl Into<String>) -> Self {
        Self {
            application: application.into(),
            ..Default::default()
        }
    }

    pub fn builder(application: impl Into<String>) -> ProviderBuilder {
        ProviderBuilder(Provider {
            application: application.into(),
            ..Default::default()
        })
    }

    pub fn load<T>(&self) -> Result<Abseil<T>>
    where
        T: Default + for<'a> Deserialize<'a>,
    {
        let location = self.location()?;
        let dir = location.path();
        let filename = self.filename.as_deref().unwrap_or(DEFAULT_FILENAME);
        let path = dir.join(filename);

        if path.exists() {
            let text = fs::read_to_string(path)?;
            return Ok(stringify::from_str(&text)?);
        }

        let legacy_path = dir.join("persist.json");
        if legacy_path.exists() {
            let text = fs::read_to_string(legacy_path)?;
            return Ok(stringify::from_str(&text)?);
        }

        Ok(Abseil::new(Default::default()))
    }

    pub fn store(&self, state: impl Serialize) -> Result<()> {
        let location = self.location()?;
        let dir = location.path();

        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }

        let filename = self.filename.as_deref().unwrap_or(DEFAULT_FILENAME);
        let path = dir.join(filename);
        let text = self.stringify(state)?;
        Ok(fs::write(path, text)?)
    }

    pub fn location(&self) -> Result<Location> {
        let directories = ProjectDirs::from(
            self.qualifier.as_deref().unwrap_or(""),
            self.organization.as_deref().unwrap_or(""),
            &self.application,
        )
        .ok_or_else(|| Error::AppData(self.clone()))?;
        Ok(Location::new(directories, self.dir))
    }

    fn stringify(&self, state: impl Serialize) -> stringify::Result<String> {
        if self.pretty {
            stringify::to_string_pretty(&Abseil::new(state))
        } else {
            stringify::to_string(&Abseil::new(state))
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(qualifier) = &self.qualifier {
            f.write_str(qualifier)?;
            f.write_str("/")?;
        }

        if let Some(organization) = &self.organization {
            f.write_str(organization)?;
            f.write_str("/")?;
        }

        f.write_str(&self.application)
    }
}

#[derive(Debug)]
pub struct ProviderBuilder(Provider);

impl ProviderBuilder {
    pub fn build(self) -> Provider {
        self.0
    }

    pub fn with_qualifier(self, qualifier: impl Into<String>) -> Self {
        Self(Provider {
            qualifier: Some(qualifier.into()),
            ..self.0
        })
    }

    pub fn with_organization(self, organization: impl Into<String>) -> Self {
        Self(Provider {
            organization: Some(organization.into()),
            ..self.0
        })
    }

    /// Instruct [`Provider`] to use pretty-printed format.
    pub fn pretty(self) -> Self {
        Self(Provider {
            pretty: true,
            ..self.0
        })
    }

    /// Set a custom filename for the storage file.
    pub fn with_filename(self, filename: impl Into<String>) -> Self {
        Self(Provider {
            filename: Some(filename.into()),
            ..self.0
        })
    }

    /// Store configuration in the config directory instead of the data directory.
    pub fn use_config_dir(self) -> Self {
        Self(Provider {
            dir: Dir::Config,
            ..self.0
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Abseil<T> {
    pub timestamp: Zoned,
    pub state: T,
}

impl<T> Abseil<T> {
    fn new(state: T) -> Self {
        Self {
            timestamp: Zoned::now(),
            state,
        }
    }

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

    use either::Either;
    use serde::{Serialize, de::DeserializeOwned};

    pub type Result<T, E = Error> = std::result::Result<T, E>;

    #[derive(Debug)]
    pub struct Error(Either<toml::de::Error, toml::ser::Error>);

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.0 {
                Either::Left(e) => e.fmt(f),
                Either::Right(e) => e.fmt(f),
            }
        }
    }

    pub fn to_string(value: &impl Serialize) -> Result<String> {
        toml::to_string(value).map_err(|e| Error(Either::Right(e)))
    }

    pub fn to_string_pretty(value: &impl Serialize) -> Result<String> {
        toml::to_string_pretty(value).map_err(|e| Error(Either::Right(e)))
    }

    pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T> {
        toml::from_str(s).map_err(|e| Error(Either::Left(e)))
    }
}
