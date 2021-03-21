# Metrics

This service records arbritrary metrics, and serves them up based on a prefix and time range.
Metrics are expected (but not enforced) to be of the form: `service.qualfifier.name` (for example,
`hosts.aura.cpu_load`). It is optimized to search for metrics based on a prefix, so we can
then query for all metrics in the last 5 days matching `hosts.aura`.

## Configuration

Two environment variables are read:

- `DBPATH` -- path the to sqlite database to record metrics in; `:memory:` will use an in-memory database
- `RUST_LOG` -- log verbosity; see the [env_logger] crate for more information

[env_logger]: https://crates.io/crates/env_logger