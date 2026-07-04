# Releasing

## Branch Strategy

- **`dev`** — active development branch. Always carries a `-dev` pre-release
  version (e.g., `X.Y.Z-dev`).
- **`main`** — release-only branch. Each commit on `main` corresponds to a
  published version.
- Merge strategy: **squash merge** from `dev` into `main`.

## Release Checklist

1. **Update CHANGELOG.md**
   - Move `[Unreleased]` items into a new `[X.Y.Z] - YYYY-MM-DD` section.
   - Add/refresh the comparison links at the bottom of the file.
   - Ensure every user-facing change is documented.

2. **Finalize version**
   - Remove the `-dev` suffix from `version` in `Cargo.toml`
     (e.g., `X.Y.Z-dev` → `X.Y.Z`).

3. **Sync the generated code**
   - The `src/generated/` tree is committed, so the published crate must match
     the schema it was generated from:
     ```sh
     cd codegen && cargo run
     cd .. && git diff --stat src/generated   # must be empty
     ```

4. **Run checks** (same gates as CI)
   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
   ```

5. **Commit on `dev`**
   - Commit the CHANGELOG and version bump together, e.g. `release: vX.Y.Z`.

6. **Squash merge into `main`**
   ```sh
   git checkout main
   git merge --squash dev
   git commit -m "Release X.Y.Z"
   ```

7. **Tag**
   ```sh
   git tag vX.Y.Z
   ```

8. **Push**
   ```sh
   git push origin main --tags
   ```

9. **Publish to crates.io**
   - Only `step-io` is published; `codegen` is a workspace-internal generator
     (`publish = false`).
   ```sh
   cargo publish -p step-io --dry-run
   cargo publish -p step-io
   ```

10. **Create the GitHub Release**
    ```sh
    gh release create vX.Y.Z --title "vX.Y.Z" --notes-file <(
      sed -n '/^## \[X\.Y\.Z\]/,/^## \[/{ /^## \[X\.Y\.Z\]/d; /^## \[/d; p; }' CHANGELOG.md
      echo "**Full Changelog**: https://github.com/elgar328/step-io/compare/vA.B.C...vX.Y.Z"
    )
    ```
    > Replace `vA.B.C` with the previous release tag.

11. **Return to `dev` and start the next cycle**
    ```sh
    git checkout dev
    git merge main
    ```
    - Bump `version` in `Cargo.toml` to the next `X.Y.Z-dev`.
    - Commit: `chore: start next development cycle (X.Y.Z-dev)`.

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
