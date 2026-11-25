# AetherLattice

Context-Aware Network State & Logic Enumeration Framework (CLI-first, Rust).

## Phase 1 Status (Foundation & Scope Model)
Implemented in Phase 1:
- `Cargo.toml` with core dependencies (`clap`, `clap_complete`, `tokio`, `serde`, `serde_json`, `serde_yaml`, `thiserror`, `tracing`, `tracing-subscriber`, `chrono`, `uuid`).
- `src/error.rs`: Stable exit codes (`0`–`8`) and domain error types (`AetherError`).
- `src/scope.rs`: Authorization scope file model (`ScopeFile`), CIDR/IP/hostname/`local://` target matching, expiry validation, and unit tests.
- `src/system.rs`: `system doctor`, `system info`, and `system completion`.
- `src/cli.rs` & `src/main.rs`: CLI grammar with global flags (`--config`, `--data-dir`, `--log-level`, `-v/--verbose`, `-q/--quiet`, `--json`, `--no-color`) and `system` / `scope` subcommands.

## Build & Test
```bash
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```

## Example Usage
```bash
lattice system doctor --verbose
lattice scope init --output ./scope.yaml
lattice scope validate --file ./scope.yaml
lattice scope explain --target 127.0.0.1:25 --file ./scope.yaml
```

## Exit Codes
- `0`: Success
- `1`: General application error
- `2`: Invalid command or arguments
- `3`: Authorization or scope denied
- `4`: Dependency or platform unavailable
- `5`: Network or operation timeout
- `6`: Partial results
- `7`: Data validation or migration failure
- `8`: Cancelled by user
