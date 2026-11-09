# Workflow: release packaging

Tool: `tools/package/package.sh` (README there), run through `cargo xtask package` (`tools/xtask`). Launcher
contract: `docs/plan/launcher_contract.md`.

| Task | Command |
|---|---|
| Build and package a release | `cargo xtask package` |
| Package the binaries already in `target/release` | `cargo xtask package --no-build` |

`cargo xtask package [args]` runs `tools/package/package.sh [args]` from the repo root; calling the script directly
does the same.

Output: `dist/rerac-<version>-<os>-<arch>/` and its `.zip` (git-ignored). The version is the one workspace version,
`version` under `[workspace.package]` in `Cargo.toml` (every crate has `version.workspace = true`). The build is a
plain `--release` build (never `--features dev`).

## The allow-list

The script packages exactly these and fails if anything else ends up in the folder:

* `rerac` (crate `rc-engine`) and `rerac-extract` (crate `rc-extract`),
* `assets/shaders/*.wgsl`,
* `rerac-manifest.json` (the launcher manifest),
* `README.txt`.

So nothing from `tools/`, `extracted/`, `work/` or `~/PS2/ratchet1/` can ship, and no disc data can be packaged. The
script also runs the packaged `rerac --version-json` to check it.

macOS and Windows (Git Bash) are tested; the Linux branch is written but untested. On Windows, `cargo xtask package`
runs Git for Windows' bash (found through `git --exec-path`), not a bare `bash`, which can be WSL's. No signing yet: a
downloaded zip is quarantined by macOS Gatekeeper.

## Where the runtime finds its shaders

The runtime loads its shaders from `assets/` next to its (symlink-resolved) executable, or from `../Resources/assets`
in a macOS `.app`; when neither exists (`cargo dev`, `target/*/rerac`) it uses the repo's `crates/rc-engine/assets`
(`main.rs` `asset_dir`).

## Cutting a release

1. Set the version in `Cargo.toml` (`[workspace.package]`, `version = "X.Y.Z"`) and add a `## vX.Y.Z[-pre]` section
   to `CHANGELOG.md`. It is player-facing: plain and short, no build internals. Commit on `main`.
2. Tag that commit with an annotated tag and push the tag:

   ```sh
   git tag -a vX.Y.Z[-pre] -m "ReRAC vX.Y.Z[-pre]"
   git push origin main vX.Y.Z[-pre]
   ```

   The tag's `X.Y.Z` must equal the workspace version (the release workflow checks); the `-pre` part (`-alpha`,
   `-beta`, `-rc…`) exists only in the tag.

## GitHub Actions (`.github/workflows/`)

Only releases run on GitHub (no CI on pushes: checks and tests run locally, `docs/workflows/testing.md`). The release
workflow builds on `macos-14` (Apple Silicon, the main tested platform), `windows-latest` and `ubuntu-22.04` (glibc 2.35;
the job installs ALSA, udev, Wayland and xkbcommon headers and `zip`), installs stable Rust with
rustup and caches with `Swatinem/rust-cache`.

* **`release.yml`** (push of a `v*` tag; or by hand with `workflow_dispatch`, input `tag`: an existing tag to release
  again, or empty to only build the packages as workflow artifacts): one build job per platform runs
  `tools/package/package.sh` (directly: on Windows a bare `bash` from `cargo xtask package` can be WSL's) and checks
  that the tag matches the workspace version; then one release job publishes `rerac-<version>-macos-arm64.zip`,
  `rerac-<version>-windows-x86_64.zip` and `rerac-<version>-linux-x86_64.zip` (each left out with a warning if its
  job failed; the Linux one is UNTESTED: built, never run) and the
  macOS `rerac-manifest.json`. The launcher picks the zip for its OS and CPU. A missing macOS package stops the
  release. It is marked a prerelease when the tag contains `-alpha`,
  `-beta` or `-rc`. The notes are the tag's `CHANGELOG.md` section, else the tag's annotation. Permissions:
  `contents: write`.

Secrets:

* `GITHUB_TOKEN` (automatic) creates the release.
* `SITE_DISPATCH_TOKEN` (optional repository secret): a token that may create `repository_dispatch` events on
  `re-rac/re-rac.github.io` (fine-grained: that repository, Contents read and write). When it exists, the release
  workflow sends the site a `release-published` event (payload: `tag`, `repository`) so the site can rebuild; without
  it the step is skipped.
