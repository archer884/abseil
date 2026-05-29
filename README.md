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

## License

MIT/Apache-2.0
