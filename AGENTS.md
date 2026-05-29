# AGENTS.md

## Project Overview

**abseil** is a Rust library crate for application state persistence. It provides a simple API to serialize application state to platform-specific data directories.

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

Two-file library: `src/lib.rs` (~260 lines) and `src/location.rs` (~40 lines). No binaries, no examples, no tests.

### Core Components

1. **`Provider`** - Main entry point. Created via `Provider::builder(app).build()` (returns `Result<Provider>`). Holds a resolved `Location` internally. Methods:
   - `load<T>()` → Deserializes state from storage file; tries direct `T` deserialization first, falls back to `Abseil<T>` for backward compat, falls back to legacy `persist.json` if new file missing, returns `Default::default()` if nothing exists. Returns `Result<T>`.
   - `store(state)` → Serializes and writes state to storage file
   - `location()` → Returns `&Location` with the resolved storage directory
   - `builder(app)` → Returns `ProviderBuilder` for configuration

2. **`Location`** (in `location.rs`) - Wraps `ProjectDirs` and `Dir` selection. Implements `Display` (shows path). Public methods:
   - `path()` → Returns `&Path` to the resolved directory (data_dir or config_dir)

3. **`ProviderBuilder`** - Fluent builder for `Provider`. Owns configuration fields; resolves `Location` at build time. Methods:
   - `with_qualifier(s)` / `with_organization(s)` → Set reverse-domain qualifiers
   - `pretty()` → Enable pretty-printing (default is compact)
   - `with_filename(s)` → Set custom filename (default is `storage.json` or `storage.toml`)
   - `use_config_dir()` → Store in config directory instead of data directory
   - `with_path(path)` → Use an explicit directory, bypassing platform-specific resolution entirely
   - `build()` → Resolves `ProjectDirs`, returns `Result<Provider>`

4. **`Abseil<T>`** - Legacy wrapper struct (`pub(crate)`) with `state: T`. Only used for backward-compatible deserialization of old payloads. Not used during serialization.

5. **`Error`** - Unified error type: `AppData(String)` | `IO(io::Error)` | `Serialization(stringify::Error)`

6. **`stringify` module** - Internal abstraction over serialization formats (see Features below)

### Module Structure

- **`location.rs`** - Contains `Dir` enum (Config/Data) and `Location` struct
  - `Dir` is `pub(crate)`
  - `Location` is `pub` with public `path()` method and `Display` impl

### Serialization Format Abstraction

The `stringify` module provides a unified interface over JSON and TOML:

- **`#[cfg(feature = "json")]`** → Uses `serde_json` directly
- **`#[cfg(all(feature = "toml", not(feature = "json")))]`** → Uses `toml` crate with a custom `enum Error { Serialization, Deserialization }` for error handling
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

### State Wrapper (Legacy)
`Abseil<T>` exists for backward-compatible deserialization only. `load()` first tries to deserialize as `T` directly; if that fails, it tries `Abseil<T>` and extracts the inner state. `store()` writes `T` directly without wrapping.

### Error Handling
- `Error` implements `Display`, `std::error::Error`, and bidirectional conversion with `io::Error`
- `From<Error> for io::Error` converts non-IO errors via `io::Error::other()`
- Custom `stringify::Error` wraps format-specific errors

### Builder Pattern
`Provider::builder(app)` returns `ProviderBuilder` (a standalone struct, not a newtype). Chain methods, then call `.build()` which returns `Result<Provider>` (location resolution happens at build time).

## Dependencies

| Crate | Purpose |
|-------|---------|
| `directories` | Platform-specific data directories |
| `serde` | Serialization framework |
| `serde_json` | JSON format (optional, default) |
| `toml` | TOML format (optional) |

## Gotchas

1. **Filename matches format**: `DEFAULT_FILENAME` const is `storage.json` for JSON feature, `storage.toml` for TOML feature.

2. **Legacy file fallback**: `load()` checks for `persist.json` if the new filename doesn't exist. This provides backward compatibility with older versions. Additionally, if direct deserialization as `T` fails, `load()` retries as `Abseil<T>` for backward compat with wrapped payloads. `store()` always writes `T` directly.

3. **Dir selection**: Storage uses `data_dir()` by default. Use `use_config_dir()` to store in `config_dir()` instead. The `Dir` enum in `location.rs` controls this behavior.

4. **Feature mutual exclusion**: `toml` feature only activates when `json` is disabled. If both are enabled, JSON wins silently.

5. **Trait bound differences**: `load<T>()` requires `T: Default + for<'a> Deserialize<'a>` with JSON, but `T: Default + DeserializeOwned` with TOML. This can cause compilation errors when switching features.

6. **Compact by default**: Providers are compact by default. Use `Provider::builder(app).pretty()` for pretty-printed output.

7. **Tests**: 13 unit tests covering roundtrip, backward compat, builder config, and errors. All use `tempfile` to avoid polluting real directories.

8. **qualifier/organization are Optional**: `ProjectDirs::from()` receives empty strings for `None` values, not the field names.

## Conventions

- **Edition**: Rust 2024 (unusual—most crates use 2021)
- **Error types**: Derive `Debug`, implement `Display` and `std::error::Error`
- **Builder pattern**: Standalone `ProviderBuilder` struct with consuming self methods; `build()` returns `Result<Provider>`
- **Module visibility**: `stringify` module is private; `location` module is public but `Dir` is `pub(crate)`; `Abseil` is `pub(crate)`
- **Type alias**: `pub type Result<T, E = Error>` for crate-local convenience

## What's Missing (for contributors)

- No examples directory
- No CI/CD configuration
- No MSRV (minimum supported Rust version) policy
- No changelog
