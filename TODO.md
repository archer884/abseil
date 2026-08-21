# TODO

- Explore locking for concurrent access. `load` and `store` are unsynchronized, so two
  processes sharing a `Provider` can race: both `load`, both `store`, and the last
  atomic rename wins, silently discarding the other process's update. The existing
  temp-file-plus-rename scheme protects against crashes, not concurrent writers.
  Possible directions: advisory file locking (`flock` via `rustix`/`fs4`) held across
  a read-modify-write cycle, or a `Provider::locked()` API returning a guard that
  derefs to the provider. Motivating case: `periodic` wraps arbitrary commands that
  may be invoked concurrently from multiple shells.
