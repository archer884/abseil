# Abseil

An easy app storage provider for Rust.

## Overview

Abseil provides a simple API to persist application state to platform-specific data directories. It handles serialization/deserialization with backward-compatible support for legacy wrapped payloads.

## Features

- **Simple API**: `Provider::builder("app").build()?.store(state)` / `provider.load()`
- **Platform-aware**: Uses OS-specific data directories via the `directories` crate
- **Configurable**: Custom filenames, pretty-printing, config vs data directory, explicit paths
- **Format support**: JSON (default) or TOML serialization
- **Legacy support**: Automatic fallback to `persist.json` for backward compatibility
- **Display**: `Provider` and `Location` implement `Display` for easy path printing

## Usage

```rust
use abseil::Provider;

// Create a provider
let provider = Provider::builder("my-app").build()?;

// Store state
provider.store(&my_state)?;

// Load state (errors if no saved state exists)
let state: MyState = provider.load()?;

// Or load with a default fallback
let state: MyState = provider.load_or_default()?;
```

### Builder Pattern

```rust
// Standard platform-specific storage
let provider = Provider::builder("my-app")
    .with_qualifier("com")
    .with_organization("example")
    .pretty()
    .with_filename("config.json")
    .use_config_dir()
    .build()?;

// Or use an explicit path (bypasses platform resolution)
let provider = Provider::builder("my-app")
    .with_path("/custom/storage/path")
    .build()?;
```

## Serialization Formats

- **JSON** (default): `serde_json`
- **TOML**: Enable with `features = ["toml"]` (must disable default JSON feature)

## Platform Directories

| OS      | Data Directory                              | Config Directory                            |
|---------|---------------------------------------------|---------------------------------------------|
| Linux   | `~/.local/share/my-app/`                    | `~/.config/my-app/`                         |
| macOS   | `~/Library/Application Support/my-app/`     | `~/Library/Application Support/my-app/`     |
| Windows | `C:\Users\{user}\AppData\Local\my-app\`    | `C:\Users\{user}\AppData\Roaming\my-app\`  |

## Changes since 0.4.0

### Breaking changes

- `load()` now returns `T` directly instead of `Abseil<T>`. A new `load_or_default()` method returns `T::default()` when no persisted state exists.
- `store()` writes `T` directly—state is no longer wrapped in `Abseil<T>`.
- Providers are compact by default. Call `.pretty()` on the builder for human-readable output.
- `PersistBuilder` renamed to `ProviderBuilder`.
- `Provider::new()` removed—use `Provider::builder()` instead.
- `Error::AppData` now holds a `String` instead of a `Provider`.
- Default storage filename is now `storage.json` (or `storage.toml`) instead of `persist.json`.
- Updated `directories` from v5 to v6; updated `toml` from v0.8 to v1.1.
- Dropped `chrono` and `either` dependencies.
- Edition bumped to Rust 2024.

### New features

- `Error::NotFound` variant for missing persisted state.
- `Location` type with a public `path()` method; `Provider::location()` returns `&Location`.
- `with_filename()` builder method to customize the storage filename.
- `with_path()` builder method to use an explicit directory, bypassing platform resolution.
- `use_config_dir()` builder method to store in the config directory instead of the data directory.
- Atomic writes via `tempfile` to prevent corruption on crash.
- `load()` falls back to legacy `persist.json` for backward compatibility with older versions.

## License

MIT/Apache-2.0
