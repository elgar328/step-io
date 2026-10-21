# Releasing

## Branch Strategy

- **`dev`** — active development branch. Always carries a `-dev` pre-release
  version (e.g., `X.Y.Z-dev`).
- **`main`** — release-only branch. Each commit on `main` corresponds to a
  published version.
- Merge strategy: **squash merge** from `dev` into `main`.

## Release Checklist

Replace `X.Y.Z` with the new version and `PREV` with the previous one
throughout.

0. **Prepare**
   - On `dev`, with a clean working tree.
   - Local `dev` and `main` match `origin` (`git fetch`, then
     `git status` on each).
   - CI is green on `dev`.
   - Choose the version (see [Versioning](#versioning-semver)): while the
     version is `0.x.y`, a breaking change bumps the minor version
     (`0.2.5` → `0.3.0`); anything else bumps the patch (`0.2.5` → `0.2.6`).

1. **Update CHANGELOG.md**
   - Move the `[Unreleased]` items into a new `## [X.Y.Z] - YYYY-MM-DD`
     section, and leave an empty `## [Unreleased]` above it.
   - Ensure every user-facing change is documented.
   - Update the links at the bottom of the file:
     ```
     [Unreleased]: https://github.com/elgar328/step-io/compare/vX.Y.Z...HEAD
     [X.Y.Z]: https://github.com/elgar328/step-io/compare/vPREV...vX.Y.Z
     ```

2. **Finalize the version**
   - Set `version` in `Cargo.toml` to the `X.Y.Z` chosen in step 0. The `-dev`
     version names the next patch, so a patch release only drops the suffix
     (`0.2.6-dev` → `0.2.6`), and a minor one changes the number as well
     (`0.2.6-dev` → `0.3.0`).

3. **Sync the generated code**
   - The `src/generated/` tree and `docs/entities.md` are committed, so the
     published crate must match the schema they were generated from:
     ```sh
     cd codegen && cargo run
     cd .. && git diff --stat src/generated docs/entities.md   # must be empty
     ```

4. **Run checks** (same gates as CI)
   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
   ```
   - Then check the minimum supported Rust version, as CI's `msrv` job does.
     The gates above update `Cargo.lock` to the new version, and this check
     needs that, so it runs after them:
     ```sh
     rustup toolchain install 1.85 --profile minimal   # once
     cargo +1.85 check --locked -p step-io --all-targets
     ```

5. **Commit and push `dev`**
   - Commit `CHANGELOG.md`, `Cargo.toml`, and `Cargo.lock` together:
     `release: vX.Y.Z`.
   - Check the package as it will be published (this needs the clean tree
     the commit leaves). Only `step-io` is published; `codegen` is a
     workspace-internal generator (`publish = false`):
     ```sh
     cargo publish -p step-io --dry-run
     ```
   - Push, and wait for CI to pass:
     ```sh
     git push origin dev
     # The run for the commit just pushed; if none is listed yet, wait a
     # moment and ask again.
     run=$(gh run list --commit "$(git rev-parse HEAD)" --json databaseId -q '.[0].databaseId')
     gh run watch "$run" --exit-status
     ```
     Pick the run by its commit: the latest run on `dev` may still be the
     previous commit's, already green. Without `--exit-status`, `gh run watch`
     exits 0 even when the run fails.
   - If CI fails, fix it on `dev` and go back to step 3. Nothing has reached
     `main` or a tag yet, so there is nothing to undo.

6. **Squash merge into `main`**
   ```sh
   git checkout main
   git merge --squash dev
   git commit -m "Release X.Y.Z"
   git diff dev main   # must print nothing
   ```

7. **Tag and push**
   ```sh
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

8. **Publish to crates.io**
   ```sh
   cargo publish -p step-io
   ```
   - Check that [crates.io](https://crates.io/crates/step-io) shows the new
     version, and, a few minutes later, that
     [docs.rs](https://docs.rs/step-io) has built its documentation.

9. **Return to `dev` and start the next cycle**
   ```sh
   git checkout dev
   git merge main --no-edit
   ```
   - Bump `version` in `Cargo.toml` to the next patch with `-dev`
     (`X.Y.Z` → `X.Y.(Z+1)-dev`); the next release may still choose a minor
     bump in step 0. Run any `cargo check` so `Cargo.lock` picks up the new
     version.
   - Commit `Cargo.toml` and `Cargo.lock`:
     `chore: start next development cycle (X.Y.(Z+1)-dev)`. CI's `msrv` job
     builds with `--locked`, so a bump without the updated `Cargo.lock` fails
     it.
   - Push: `git push origin dev`.

### If a published release is broken

A version on crates.io cannot be replaced or deleted. Yank it, so that new
projects no longer resolve to it, then release the fix as a new patch
version:

```sh
cargo yank step-io --version X.Y.Z
```

## Versioning (SemVer)

This project follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** (`X.0.0`) — incompatible API changes.
- **MINOR** (`0.X.0`) — new functionality, backwards compatible.
- **PATCH** (`0.0.X`) — backwards-compatible bug fixes.

While the version is `0.x.y`, minor bumps may include breaking changes (the
crate is experimental).

## CHANGELOG Guidelines

Follow the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

### Categories

- **Added** — new features.
- **Changed** — changes to existing functionality.
- **Deprecated** — features that will be removed in upcoming releases.
- **Removed** — features that have been removed.
- **Fixed** — bug fixes.
- **Security** — vulnerability fixes.

### Rules

- Write entries from the user's perspective, not the developer's.
- Each entry is a concise, complete sentence.
- Most recent release goes first.
- Always keep an `[Unreleased]` section at the top for ongoing work.
- Within a release, order the category sections as listed under
  [Categories](#categories) above (Added, Changed, Deprecated, Removed, Fixed,
  Security); omit any category that has no entries.
