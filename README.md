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

```rust,no_run
use abseil::Provider;

# use serde::{Deserialize, Serialize};
#
# #[derive(Default, Serialize, Deserialize)]
# struct MyState { count: u32 }
#
# let my_state = MyState { count: 1 };
#
// Create a provider
let provider = Provider::builder("my-app").build()?;

// Store state
provider.store(&my_state)?;

// Load state (errors if no saved state exists)
let state: MyState = provider.load()?;

// Or load with a default fallback
let state: MyState = provider.load_or_default()?;
#
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Builder Pattern

```rust,no_run
# use abseil::Provider;
#
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
#
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Serialization Formats

- **JSON** (default): `serde_json`
- **TOML**: Enable with `features = ["toml"]` (must disable default JSON feature)

## Concurrent Access

By default, `load()` and `store()` are unsynchronized: two processes sharing a
storage location can race, and the last write silently wins. The opt-in
`locking` feature (no additional dependencies, requires Rust 1.89+) adds
advisory cross-process file locking so cooperating writers can serialize
load-modify-store cycles:

```rust
# use abseil::Provider;
# use serde::{Deserialize, Serialize};
#
# #[derive(Default, Serialize, Deserialize)]
# struct Count { total: u32 }
#
# #[cfg(feature = "locking")]
# fn lock_and_update(provider: &Provider) -> abseil::Result<()> {
#
// Atomic read-modify-write under the lock:
let state: Count = provider.update(|c: &mut Count| {
    c.total += 1;
    Ok(())
})?;

// Or hold a lock guard manually:
let guard = provider.lock()?;          // or try_lock()? for non-blocking
let mut state: Count = guard.load_or_default()?;
state.total += 1;
guard.store(&state)?;
#
# Ok(())
# }
#
# #[cfg(feature = "locking")]
# {
#     let dir = tempfile::tempdir().unwrap();
#     let provider = Provider::builder("my-app").with_path(dir.path()).build().unwrap();
#     lock_and_update(&provider).unwrap();
# }
```

Locks are taken on a sidecar `<filename>.lock` file, released on drop or
process death, and advisory—writers that don't take the lock are unaffected.

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
