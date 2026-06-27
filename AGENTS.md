# AGENTS.md

Onboarding notes for contributors to **abseil**, a small Rust library crate for
persisting application state to platform-specific data directories.

## Project Overview

**abseil** serializes a value of any `Serialize`/`Deserialize` type to a JSON
or TOML file under the OS-appropriate data directory, and reads it back. The
public surface is two types (`Provider`, `ProviderBuilder`) and a `Location`
view of the resolved path.

- **Repository**: https://github.com/archer884/abseil
- **License**: MIT/Apache-2.0 dual license
- **MSRV**: not formally declared; edition 2024 features are used.

## Build & Test

```bash
cargo check                                  # type-check
cargo build                                  # build the library
cargo test                                   # 23 unit tests + 4 doctests (default features)
cargo test --features xdg-migration          # 34 unit tests + 4 doctests
cargo test --no-default-features --features toml  # exercise the TOML module
cargo fmt -- --check                         # verify formatting
cargo clippy --all-targets                   # lint (also under --features ...)
cargo doc --no-deps                          # build docs; should be warning-free
```

There is no CI configuration in the repo. There is no `rustfmt.toml` or
`clippy.toml`; the crate uses defaults for both. Both `cargo fmt` and
`cargo clippy --all-targets` (with and without `xdg-migration`) should be
clean before pushing.

## Repository Layout

```
abseil/
├── Cargo.toml          # 4 features: default = ["json"], plus json, toml, xdg-migration
├── README.md           # crate-level docs (also #![doc = include_str!("../README.md")])
├── AGENTS.md           # this file
├── LICENSE-MIT
├── LICENSE-APACHE
├── .gitignore          # /target, /Cargo.lock
└── src/
    ├── lib.rs          # Provider, ProviderBuilder, Error, Abseil<T>, stringify module, tests
    ├── location.rs     # Location, Root, RootLazy, XdgKind, xdg_path helper
    └── migration.rs    # migrate_legacy_to_xdg helper (gated on xdg-migration feature)
```

`src/lib.rs` is ~950 lines including inline tests. `src/location.rs` is ~95
lines. `src/migration.rs` is ~40 lines and is the entire `xdg-migration`
feature. There is no `examples/` or `benches/` directory.

## Architecture

The crate is a thin facade over three concerns: path resolution, serialization,
and atomic file I/O.

### Public types

- **`Provider`** — owns a resolved `Location` and a `filename`. `store` writes
  state via a `tempfile::NamedTempFile` then renames; `load` reads with a
  three-layer fallback (see "Backward compatibility" below).
- **`ProviderBuilder`** — standalone (not a newtype) builder. Owns qualifier,
  organization, application, pretty flag, filename override, and a `RootLazy`
  variant that captures the path-resolution strategy. `build()` resolves the
  path and returns a `Provider`.
- **`Location`** — wraps the resolved root. Public `path() -> &Path` and
  `Display`. Lives in `pub mod location`.
- **`Error`** — `enum Error { AppData(String), IO(io::Error), NotFound,
  Serialization(stringify::Error) }` with `From<io::Error>` and
  `From<stringify::Error>` impls plus a `From<Error> for io::Error` impl.

### Crate-internal types

- **`RootLazy`** (in `location.rs`) — the builder-time variant that captures
  *intent*: `PlatformData`, `PlatformConfig`, `XdgData`, `XdgConfig`, or
  `Path(PathBuf)`. Encodes both the data-vs-config choice and the
  platform-vs-XDG choice in one tag, so the two builder methods
  (`use_config_dir` / `use_xdg_layout`) compose cleanly in either order.
- **`Root`** (in `location.rs`) — the resolved variant: holds either a
  `ProjectDirs`, an `Xdg(PathBuf)`, or a `Path(PathBuf)`. The XDG path is
  resolved eagerly at `build()` time, so `Location::path()` is a simple
  match.
- **`XdgKind`** (in `location.rs`) — `Data` or `Config`. Knows its env var
  (`XDG_DATA_HOME` / `XDG_CONFIG_HOME`) and its default subdir (`.local/share`
  / `.config`).
- **`Abseil<T>`** (in `lib.rs`) — legacy wrapper. `pub(crate) struct
  Abseil<T> { state: T }`. Used only by `try_load_file` to deserialize old
  wrapped payloads. Never written by current code.
- **`stringify` module** (in `lib.rs`, private) — uniform surface over JSON
  and TOML. Two feature-gated implementations; see "Features" below.

### Path resolution

`build()` matches on the `RootLazy` variant:

| Variant | Behavior |
|---|---|
| `Path` | Use the explicit path. No resolution. |
| `PlatformData` / `PlatformConfig` | `ProjectDirs::from(qualifier, organization, application)` then take `data_dir()` or `config_dir()`. |
| `XdgData` / `XdgConfig` | On macOS: read `XDG_DATA_HOME` (or `XDG_CONFIG_HOME`) with `~/.local/share` / `~/.config` fallback via `HOME`. On other OS: use `ProjectDirs` so the resolved path matches the platform default. |

The `XdgData`/`XdgConfig` branches on non-macOS are an intentional
"identity" path: `use_xdg_layout()` is documented as a no-op on non-macOS
and the resolution reflects that.

## Public API Quick Reference

### `Provider`

```rust
let provider = Provider::builder("my-app").build()?;
provider.store(&state)?;        // atomic write
let state: T = provider.load()?;          // returns T directly
let state: T = provider.load_or_default()?; // T::default() on NotFound
provider.location().path();     // &Path to the resolved dir
format!("{provider}");          // same as location().path()
```

### `ProviderBuilder` methods (all consume and return `self`)

| Method | Effect |
|---|---|
| `with_qualifier(s)` | Sets `qualifier` component of the reverse-domain identifier. |
| `with_organization(s)` | Sets `organization` component. |
| `pretty()` | Pretty-print on `store`. Default is compact. |
| `with_filename(s)` | Override the storage filename. Default is `storage.json` (or `.toml`). |
| `use_config_dir()` | Switch data→config for both `use_xdg_layout` and the default layout. Idempotent; composes with `use_xdg_layout`. |
| `use_xdg_layout()` | On macOS, resolve to `$XDG_DATA_HOME/<app>` / `$XDG_CONFIG_HOME/<app>`. No-op on other OS. |
| `with_migrate()` | (Gated on `xdg-migration`.) On macOS, before the provider is built, move the primary file (or, failing that, `persist.json`) from the legacy `~/Library/Application Support/<app>/` to the XDG path. No-op without `use_xdg_layout` and outside macOS. Always present in the API; a no-op when the feature is disabled. |
| `with_path(p)` | Use an explicit directory. Overrides every other setting including `use_xdg_layout` and `use_migrate`. |
| `build() -> Result<Provider>` | Resolve the path. Returns `Error::AppData` if platform resolution fails and no explicit path was set. |

`use_config_dir` and `use_xdg_layout` both preserve the other dimension, so:

```rust
Provider::builder("a").use_xdg_layout().use_config_dir().build()
Provider::builder("a").use_config_dir().use_xdg_layout().build()
```

are equivalent. Both produce `RootLazy::XdgConfig`.

## Features

```toml
[features]
default = ["json"]
json    = ["dep:serde_json"]   # default
toml    = ["dep:toml"]         # only effective when json is disabled
xdg-migration = []             # adds the migration feature; no new deps
```

- **json / toml** — select the serialization backend. The `stringify` module
  is private and has two feature-gated implementations; only one is compiled
  in. JSON wins if both are enabled (TOML is gated on `not(feature = "json")`).
- **xdg-migration** — gates the entire `src/migration.rs` module and the
  `migrate` field / setter on `ProviderBuilder`. The setter `with_migrate()`
  is always present in the API surface (so user code compiles with or without
  the feature) but becomes a no-op when the feature is off.

### Trait bound differences across features

`load::<T>()` requires `T: for<'a> Deserialize<'a>` under the JSON feature
and `T: DeserializeOwned` under TOML. `load_or_default::<T>()` additionally
requires `T: Default`. This can cause user code to compile under one feature
combination and fail under another — the README calls this out.

## Key Patterns

### Backward compatibility on read

`Provider::load` is built around a three-layer fallback (all transparent to
the user):

1. Read the primary file (the configured filename, or the format default).
2. If the primary read fails with `io::ErrorKind::NotFound`, try
   `persist.json` in the same directory. (This handles upgrade from pre-0.5
   abseil, which always used `persist.json`.)
3. If the primary deserialize fails, try deserializing as `Abseil<T>` and
   extract the inner `state` field. (This handles pre-0.5 payloads that
   wrapped state in `{"state": ...}`.)

`store` always writes the bare `T` (no wrapper). The `Abseil<T>` struct is
only used for the read fallback.

The cost on the happy path (primary file exists and parses as `T`) is
zero — no extra `fs::exists`, no extra stat. The fallback paths live in
code that's never entered.

### Atomic writes

`store` writes to a `NamedTempFile` in the destination directory, then
renames to the target. Crash-safety: a torn write leaves the temp file and
the previous good state.

### XDG layout on macOS

`use_xdg_layout()` only changes behavior on macOS. On macOS, the XDG path
is computed from the `XDG_DATA_HOME` / `XDG_CONFIG_HOME` env vars (with
`~/.local/share` / `~/.config` defaults via `HOME`). On every other OS,
the resolved path is identical to the platform default — the method is a
no-op there, called out explicitly in the doc comment so users don't
expect a behavior change on Linux/Windows.

The pure `xdg_path` helper takes its inputs (env values, `HOME`) as
parameters, which lets the test suite exercise every branch without
mutating process environment.

### Library migration (xdg-migration feature)

`migrate_legacy_to_xdg` (in `src/migration.rs`) is the entire migration
feature:

1. If `<legacy>/<primary_filename>` exists and `<xdg>/<primary_filename>`
   does not, `fs::rename` it.
2. Else, if `<legacy>/persist.json` exists and `<xdg>/persist.json` does
   not, `fs::rename` it.
3. Else, do nothing.

Never overwrites the destination. Idempotent by construction. The
"primary wins, persist.json is fallback" order mirrors `load()`'s read
order, so a user with both files at the legacy location will end up
with the primary at XDG and `persist.json` left behind at legacy (which
is fine — `load()` finds the primary first).

The hook in `build()` is gated on `migrate && Root::Xdg && macos`. Every
other configuration (other OS, no `use_xdg_layout`, `with_path` set,
feature disabled) skips the migration entirely. The setter `with_migrate`
is *always* present, so user code compiles in either feature
configuration; the `#[cfg(feature = "xdg-migration")]` decides whether the
field exists.

## Gotchas

1. **Filename matches format** — `DEFAULT_FILENAME` is `storage.json` under
   the JSON feature, `storage.toml` under TOML. The `stringify` module's
   `to_string` / `from_str` swap in the right format-specific functions.

2. **`with_path` wins over everything** — including `use_xdg_layout` and
   `with_migrate`. The `Root::Path` arm short-circuits both the XDG
   resolution and the migration.

3. **`use_xdg_layout` is macOS-only behavior** — on other OS it's a
   silent no-op (the resolved path matches the platform default). Don't
   tell users it changes anything on Linux/Windows.

4. **`xdg-migration` only fires on macOS with `use_xdg_layout()`** — even
   with the feature enabled, `with_migrate()` alone (no `use_xdg_layout`)
   does nothing because the migration branch in `build()` only runs when
   the root is `Root::Xdg`.

5. **`HOME` is read at `build()` time** — and only on macOS. If a user
   changes `HOME` between calls, they'll get different XDG paths.

6. **Empty `XDG_DATA_HOME` / `XDG_CONFIG_HOME` are treated as unset** —
   `.filter(|s| !s.is_empty())` in `build()`. This matches the XDG
   spec's "ignore empty values" rule.

7. **Tests use `tempfile`** — every test that touches the filesystem
   creates a `tempfile::tempdir()`. Don't write tests that touch the
   real user data directory. There is no fixture-cleanup pattern beyond
   `tempdir`'s `Drop`.

8. **qualifier/organization default to empty strings** — `ProjectDirs::from`
   receives `""` when the corresponding builder field is `None`, not
   the field name. This matters when debugging `Error::AppData`.

9. **TOML is mutually exclusive with JSON in practice** — `toml` is
   gated on `not(feature = "json")` in the feature table. Enabling
   both silently selects JSON.

10. **No CI** — there's no `.github/`, no `Makefile`, no shell scripts.
    Run `cargo fmt --check`, `cargo clippy --all-targets`, and
    `cargo test` (with and without `--features xdg-migration`) locally
    before pushing.

## Conventions

- **Edition**: Rust 2024.
- **Builder**: standalone `ProviderBuilder` (not a newtype), consuming
  self methods, `build() -> Result<Provider>`.
- **Error types**: `Debug` + `Display` + `std::error::Error`. `Error`
  has bidirectional `From` with `io::Error`.
- **Module visibility**: `stringify` is private; `migration` is private
  and gated on the feature; `location` is `pub` (only `Location` and
  its `path()` method are externally visible; `Root`, `RootLazy`,
  `XdgKind`, and `xdg_path` are `pub(crate)`).
- **No comments in code unless asked** — the codebase has rustdoc on
  every public item, and a small amount of inline `//` where the
  intent isn't obvious from the code. Don't add narrative comments.
- **Type alias**: `pub type Result<T, E = Error>` for crate-local
  convenience.

## Common Tasks

### Adding a new builder method

1. Add the field to `ProviderBuilder` (in `src/lib.rs`).
2. If it affects path resolution, add a variant to `RootLazy` (in
   `src/location.rs`) and `Root`, and an arm in `Location::path()`.
3. Implement the setter. If it composes with `use_config_dir` /
   `use_xdg_layout`, write it as a transition on the existing
   `RootLazy` so the two methods compose in either order (see
   `use_config_dir` for the pattern).
4. Add the arm in `build()` to resolve the new variant.
5. Add a doc comment with at least one example. Doctests must be
   wrapped in `fn main() -> Result<(), Box<dyn std::error::Error>>`
   to compile.
6. Add a unit test (or two) in the `tests` module at the bottom of
   `lib.rs`. Use `tempfile::tempdir()` for any test that touches the
   filesystem.

### Adding a new feature

1. Add the feature to `Cargo.toml` under `[features]`.
2. Gate the new module with `#[cfg(feature = "...")]` at the `mod`
   declaration in `lib.rs`.
3. Keep public API surface stable: prefer setters that exist regardless
   of the feature flag (with a no-op fallback under `#[cfg(not(...))]`)
   so user code compiles in both configurations.
4. Gate tests with `#[cfg(feature = "...")]`.

### Adding a doctest

Doctests live in the README (`#![doc = include_str!("../README.md")]`).
Wrap every snippet in `fn main() -> Result<(), Box<dyn std::error::Error>>`
so `?` is allowed. Verify with `cargo test --doc` (and
`cargo test --doc --features xdg-migration` if the snippet uses a
gated API).

## Open Questions / Future Work

- **Phase 2 of the XDG work** (deferred). Gating the `persist.json`
  read fallback in `load()` behind a feature, and adding a load-time
  rename so the legacy file is moved to the primary filename when it's
  read. Discussed in the conversation log; not implemented because the
  fallback has no meaningful runtime cost on the happy path.
- **No MSRV policy.** Worth declaring one.
- **No CI.** A GitHub Actions workflow that runs `cargo fmt --check`,
  `cargo clippy --all-targets`, `cargo test`, and
  `cargo test --features xdg-migration` on Linux would catch the
  "feature-off build is broken" class of bug.
- **No examples directory.** A short `examples/` showing the
  `use_xdg_layout` + `with_migrate` workflow on macOS would help
  downstream users.
- **`Location` was re-exported as `pub` recently** — the
  `pub mod location;` change (in this branch) makes the type reachable
  to downstream users. Worth verifying that nothing in the
  `pub(crate)` items of that module accidentally leaks.
