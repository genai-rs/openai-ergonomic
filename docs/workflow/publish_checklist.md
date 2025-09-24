# Release & Publish Checklist

This checklist guides maintainers through preparing and publishing a new version of `openai-ergonomic`. Complete every step in order; skip nothing. Record the release date and outcome at the end for future reference.

## 1. Preflight Verification

- [ ] Confirm `main` is healthy (CI green, no pending required reviews)
- [ ] Review `PLAN.md` and `TODO.md` for unfinished critical items
- [ ] Ensure `CHANGELOG.md` contains entries for the upcoming release
- [ ] Bump crate version in `Cargo.toml` (following semver)
- [ ] Update dependency versions if release-plz flagged changes
- [ ] Run the full pre-commit suite locally:
  ```bash
  cargo fmt --all
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all
  cargo build --all-targets
  cargo build --examples
  cargo doc --no-deps
  ```
- [ ] Validate examples compile and run where practical (`cargo run --example ... -- --help`)

## 2. Dry Run & Validation

- [ ] Generate release artifacts locally:
  ```bash
  cargo publish --dry-run
  ```
- [ ] Confirm docs build for docs.rs (including feature combos when relevant):
  ```bash
  RUSTFLAGS="-D warnings" cargo doc --no-deps --all-features
  ```
- [ ] Execute `release-plz` dry run for GitHub automation:
  ```bash
  release-plz release-pr --dry-run
  release-plz manifest --dry-run
  ```
- [ ] Inspect generated changelog/version bump for accuracy
- [ ] Double-check licensing headers and attribution policy compliance

## 3. Release Execution

- [ ] Create/refresh the release PR (`release-plz release-pr`)
- [ ] Review PR output (version, changelog, dependency updates)
- [ ] Merge the release PR once approved
- [ ] Tag the release locally (if not handled automatically):
  ```bash
  git tag -a vX.Y.Z -m "Release vX.Y.Z"
  git push origin vX.Y.Z
  ```
- [ ] Publish to crates.io (manual trigger if automation disabled):
  ```bash
  cargo publish --allow-dirty # use only if release-plz already staged artifacts
  ```
- [ ] Monitor publish output for errors before proceeding

## 4. Post-Release Follow-up

- [ ] Verify the release on crates.io (version, readme, documentation link)
- [ ] Check docs.rs build status (retry if failed)
- [ ] Update `CHANGELOG.md` with release date if needed
- [ ] Announce release:
  1. Post update in project communication channels (Slack/Discord/Mailing list)
  2. Tweet/Blog if applicable (include highlights)
- [ ] Close or update issues linked to the release
- [ ] File follow-up tasks in `TODO.md` for next milestones
- [ ] Record release summary here:
  ```markdown
  - vX.Y.Z – YYYY-MM-DD – outcome / notable notes
  ```

## Reference Commands

Quick commands for common release tasks:

```bash
# Refresh release PR
release-plz release-pr

# Update manifests (Cargo.toml, CHANGELOG) after adjustments
release-plz manifest

# Yank an accidental release
cargo yank --vers X.Y.Z
```

Keep this checklist synchronized with `PLAN.md` and `TODO.md`. Update it whenever process adjustments are made.
