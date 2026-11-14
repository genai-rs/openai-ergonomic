# Development

The [contributing guide](../CONTRIBUTING.md) has the workflow, the commit format and the pull
request process; this page has the commands.

## Checks

What CI runs (`.github/workflows/ci.yml`, the shared genai-rs `rust-ci.yml`), with the MSRV from
`Cargo.toml`:

```bash
MSRV=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo +"$MSRV" clippy --all-targets --all-features -- -D warnings
cargo +"$MSRV" test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo semver-checks
```

and the [WebAssembly](webassembly.md#tests) clippy and Node.js suite.

## Tests

- **Unit tests** live next to the code (`src/**`).
- **Integration tests** (`tests/*.rs`) run the client against [mockito](https://docs.rs/mockito)
  servers: builders, providers, authentication, streaming, interceptors, uploads, webhooks. No
  API key is needed.
- **Documentation tests** compile the examples of the rustdoc, the README and the pages under
  `docs/` (`src/lib.rs` includes the pages in a `#[cfg(doctest)]` module), so a moved or renamed
  API breaks the build instead of the documentation. Most are `no_run`: they need no credentials.
- **WebAssembly**: `tests/wasm.rs`, in Node.js (see [WebAssembly](webassembly.md#tests)).
- **Smoke tests**: `tests/smoke.rs` calls the real API. Its tests are `#[ignore]`d and compile
  with the others; run them with the key of a test project:
  `OPENAI_TEST_API_KEY=... cargo test --test smoke -- --ignored` (or `cargo make test-smoke`),
  with `OPENAI_TEST_MODEL` and `OPENAI_TEST_BASE_URL` to pick the model and endpoint.

`cargo make test-unit`, `test-integration` and `test-doc` (`Makefile.toml`) run each group.
`scripts/check-msrv.sh` builds and tests with the MSRV toolchain.

## Local CI

The genai-rs repositories can attest a commit that passed the CI checks on a developer machine,
so that the heavy CI jobs are skipped for it. From a checkout of this repository, with a clone of
[`genai-rs/.github`](https://github.com/genai-rs/.github) next to it:

```bash
bash ../.github/scripts/local-ci.sh --push    # a new, unpushed commit
bash ../.github/scripts/local-ci.sh --rerun   # a commit already pushed
```

It runs the checks above on stable and on the MSRV toolchain (and the WebAssembly clippy and
Node.js suite when the `wasm32-unknown-unknown` target, Node.js and a `wasm-bindgen-test-runner`
of `Cargo.lock`'s `wasm-bindgen` version are installed), then posts a `local-ci` commit status
that CI's gate honours. Pushes to `main` always run the full CI.
