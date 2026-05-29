# AGENTS.md

## Project Overview

**abseil** is a Rust library crate for application state persistence. It provides a simple API to serialize application state to platform-specific data directories with automatic timestamps.

**Repository**: https://github.com/archer884/abseil  
**License**: MIT/Apache-2.0 dual license

## Essential Commands

```bash
cargo check          # Type check without building
cargo build          # Build the library
cargo test           # Run tests (currently none exist)
cargo fmt            # Format code
cargo fmt -- --check # Verify formatting
cargo clippy         # Lint (if clippy is installed)
```

**Note**: No CI configuration exists. No custom rustfmt.toml or clippy.toml—uses defaults.

## Architecture

Two-file library: `src/lib.rs` (~275 lines) and `src/location.rs` (~33 lines). No binaries, no examples, no tests.

### Core Components

1. **`Provider`** - Main entry point. Configured with app name (and optional qualifier/organization). Methods:
   - `load<T>()` → Deserializes state from storage file in data dir, falls back to legacy `persist.json` if new file missing, returns `Default::default()` if neither exists
   - `store(state)` → Serializes and writes state to storage file
   - `location()` → Returns `Result<Location>` with the resolved storage directory
   - `builder(app)` → Returns `ProviderBuilder` for advanced configuration

2. **`Location`** (in `location.rs`) - Wraps `ProjectDirs` and `Dir` selection. Public methods:
   - `path()` → Returns `&Path` to the resolved directory (data_dir or config_dir)

3. **`ProviderBuilder`** - Fluent builder for `Provider`. Methods:
   - `with_qualifier(s)` / `with_organization(s)` → Set reverse-domain qualifiers
   - `pretty()` → Enable pretty-printing (default is compact)
   - `with_filename(s)` → Set custom filename (default is `storage.json` or `storage.toml`)
   - `use_config_dir()` → Store in config directory instead of data directory

4. **`Abseil<T>`** - Wrapper struct with `timestamp: Zoned` and `state: T`. All persisted data is wrapped in this.

5. **`Error`** - Unified error type: `AppData(Provider)` | `IO(io::Error)` | `Serialization(stringify::Error)`

6. **`stringify` module** - Internal abstraction over serialization formats (see Features below)

### Module Structure

- **`location.rs`** - Contains `Dir` enum (Config/Data) and `Location` struct
  - `Dir` is `pub(crate)` 
  - `Location` is `pub` with public `path()` method

### Serialization Format Abstraction

The `stringify` module provides a unified interface over JSON and TOML:

- **`#[cfg(feature = "json")]`** → Uses `serde_json` directly
- **`#[cfg(all(feature = "toml", not(feature = "json")))]`** → Uses `toml` crate with `either::Either` for error handling
- **Mutual exclusion**: TOML is only active when JSON feature is disabled

## Features

```toml
[features]
default = ["json"]    # JSON by default
json = ["dep:serde_json"]
toml = ["dep:toml"]   # Only works when json is disabled
```

**Gotcha**: The TOML feature uses `DeserializeOwned` (not `Deserialize<'a>`) due to toml crate requirements. This means the `load<T>()` method's trait bounds change depending on active features.

## Key Patterns

### File Storage Location
- Uses `directories::ProjectDirs` for platform-specific paths
- `Provider::location()` resolves to `Location` which wraps `ProjectDirs` + `Dir` selection
- Default is `data_dir()`, but can be configured to use `config_dir()` via `use_config_dir()`
- Default filename matches format: `storage.json` for JSON, `storage.toml` for TOML (set via `DEFAULT_FILENAME` const)
- Custom filename can be set via `with_filename()`

### State Wrapper
All state is wrapped in `Abseil<T>` which automatically adds `Zoned::now()` timestamp on save. Users access the inner state via `abseil.into_inner()`.

### Error Handling
- `Error` implements `Display`, `std::error::Error`, and bidirectional conversion with `io::Error`
- `From<Error> for io::Error` converts non-IO errors via `io::Error::other()`
- Custom `stringify::Error` wraps format-specific errors

### Builder Pattern
`Provider::builder()` returns `ProviderBuilder` (newtype over `Provider`). Chain methods, then call `.build()`.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `jiff` | Timestamps in `Abseil<T>` |
| `directories` | Platform-specific data directories |
| `either` | TOML error type unification |
| `serde` | Serialization framework |
| `serde_json` | JSON format (optional, default) |
| `toml` | TOML format (optional) |

## Gotchas

1. **Filename matches format**: `DEFAULT_FILENAME` const is `storage.json` for JSON feature, `storage.toml` for TOML feature.

2. **Legacy file fallback**: `load()` checks for `persist.json` if the new filename doesn't exist. This provides backward compatibility with older versions. `store()` always writes to the new filename.

3. **Dir selection**: Storage uses `data_dir()` by default. Use `use_config_dir()` to store in `config_dir()` instead. The `Dir` enum in `location.rs` controls this behavior.

4. **Feature mutual exclusion**: `toml` feature only activates when `json` is disabled. If both are enabled, JSON wins silently.

5. **Trait bound differences**: `load<T>()` requires `T: Default + for<'a> Deserialize<'a>` with JSON, but `T: Default + DeserializeOwned` with TOML. This can cause compilation errors when switching features.

6. **Compact by default**: `Provider::new()` creates compact provider. Use `Provider::builder(app).pretty()` for pretty-printed output.

7. **No tests exist**: The library has zero unit tests or doc tests. Any changes should include tests.

8. **qualifier/organization are Optional**: `ProjectDirs::from()` receives empty strings for `None` values, not the field names.

## Conventions

- **Edition**: Rust 2024 (unusual—most crates use 2021)
- **Error types**: Derive `Debug`, implement `Display` and `std::error::Error`
- **Builder pattern**: Newtype wrapper `ProviderBuilder(Provider)` with consuming self methods
- **Module visibility**: `stringify` module is private; `location` module is public but `Dir` is `pub(crate)`
- **Type alias**: `pub type Result<T, E = Error>` for crate-local convenience

## What's Missing (for contributors)

- No unit tests
- No doc comments on public items
- No examples directory
- No CI/CD configuration
- No MSRV (minimum supported Rust version) policy
- No changelog

## Review comments

1.  Dir  is  pub(crate)  but imported in  lib.rs  - If  location.rs  is truly a separate module, the import  use location::{Dir, Location}  suggests it's nested under the crate root, not a standalone file. The AGENTS.md says "two- file" but it's really one module split across files.

2. No tests - AGENTS.md calls this out, but it's worth repeating. Core logic (fallback, dir selection, serialization) should have coverage.

3. `Error::AppData(Provider)``  is unusual - Storing the entire  Provider  in the error is heavy. Consider storing just the application name or a display string.

4. `Either` for TOML errors - Works, but  Box<dyn Error>  or a custom enum would be more explicit. The `either` crate dependency exists only for this.
