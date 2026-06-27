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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use abseil::Provider;
    use serde::{Deserialize, Serialize};

    #[derive(Default, Serialize, Deserialize)]
    struct MyState {
        name: String,
        count: u32,
    }

    // Create a provider
    let provider = Provider::builder("my-app").build()?;

    // Store state
    let my_state = MyState {
        name: "example".into(),
        count: 1,
    };
    provider.store(&my_state)?;

    // Load state (errors if no saved state exists)
    let _state: MyState = provider.load()?;

    // Or load with a default fallback
    let _state: MyState = provider.load_or_default()?;

    Ok(())
}
```

### Builder Pattern

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use abseil::Provider;

    // Standard platform-specific storage
    let _provider = Provider::builder("my-app")
        .with_qualifier("com")
        .with_organization("example")
        .pretty()
        .with_filename("config.json")
        .use_config_dir()
        .build()?;

    // On macOS, use XDG-style paths instead of ~/Library/Application Support.
    // No-op on other platforms.
    let _provider = Provider::builder("my-app")
        .use_xdg_layout()
        .use_config_dir()
        .build()?;

    // Or use an explicit path (bypasses platform resolution)
    let _provider = Provider::builder("my-app")
        .with_path("/custom/storage/path")
        .build()?;

    Ok(())
}
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

### XDG layout on macOS

If you prefer XDG-style paths over `~/Library/Application Support` on macOS, call `use_xdg_layout()` on the builder. This resolves:

- **Data**: `$XDG_DATA_HOME/my-app` (defaulting to `~/.local/share/my-app`)
- **Config**: `$XDG_CONFIG_HOME/my-app` (defaulting to `~/.config/my-app`)

The `XDG_DATA_HOME` and `XDG_CONFIG_HOME` environment variables are respected when set, matching the [XDG Base Directory Specification]. `use_xdg_layout()` is a no-op on non-macOS platforms; use `with_path()` if you need an explicit override on those targets.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use abseil::Provider;

    // On macOS: data dir resolves to ~/.local/share/my-app
    // On other platforms: identical to the default platform path
    let _provider = Provider::builder("my-app")
        .use_xdg_layout()
        .build()?;

    // .use_config_dir() would switch to ~/.config/my-app on macOS
    Ok(())
}
```

[XDG Base Directory Specification]: https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

### Migration from ~/Library

If you ship `use_xdg_layout()` to users who previously stored data at the legacy macOS path (`~/Library/Application Support/...`), those users will silently see an empty state on first launch. To relocate their data on first run, enable the `xdg-migration` Cargo feature and call `with_migrate()` on the builder:

```toml
[dependencies]
abseil = { version = "0.5", features = ["xdg-migration"] }
```

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use abseil::Provider;

    let _provider = Provider::builder("my-app")
        .use_xdg_layout()
        .with_migrate()
        .build()?;

    Ok(())
}
```

On macOS, `build()` will then:

1. Check whether the XDG path is empty.
2. If it is, look for the primary storage file at the legacy `~/Library/Application Support/<app>/` location and rename it to the XDG path.
3. If the primary isn't there, fall back to the legacy `persist.json` file and rename that to `<xdg>/persist.json`. The existing `load()` fallback finds it.
4. If a file with the target name already exists at the XDG path, leave it alone — the destination always wins.

The migration is idempotent: subsequent `build()` calls see the file at the XDG path and do nothing.

**Recommended lifecycle**: enable the feature and call `with_migrate()` for a few releases while users upgrade, then drop the call (and disable the feature) once enough time has passed. The library will continue to function identically; only the one-time move stops happening.

The migration is a no-op when:

- the `xdg-migration` feature is disabled,
- `use_xdg_layout()` is not called,
- running on non-macOS (the XDG layout is itself a no-op there), or
- neither the primary file nor `persist.json` exists at the legacy location.

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
- `use_xdg_layout()` builder method to resolve to XDG-style paths (`~/.local/share`, `~/.config`) on macOS instead of `~/Library/Application Support`. No-op on other platforms.
- `xdg-migration` Cargo feature with `with_migrate()` builder method to one-shot move storage data from `~/Library/Application Support` to the XDG path on macOS. Designed to be enabled for a few releases and then disabled.
- Atomic writes via `tempfile` to prevent corruption on crash.
- `load()` falls back to legacy `persist.json` for backward compatibility with older versions.

## License

MIT/Apache-2.0
