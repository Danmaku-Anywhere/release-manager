# Agent context: release-manager (Danmaku Builds)

## Purpose

A self-updating desktop app (Tauri 2) that downloads extension builds from the `Danmaku-Anywhere/danmaku-anywhere` releases and manages which cached build is the "active" unpacked folder loaded into `chrome://extensions`. It is versioned and released independently of the extension. Linux and macOS self-update; Windows ships an installer and a portable exe with no in-app update and no signing key. Install and platform notes for users are in `README.md`.

## Architecture

- `src/` is a one-screen React frontend that talks to the backend over Tauri IPC (`src/api.ts`, `src/updater.ts`). There is no HTTP server and no GitHub token; the repo is public and unauthenticated requests suffice.
- `src-tauri/core/` (`release-manager-core`) is pure logic with no Tauri dependency, so its tests run without the webkit libraries. Fallible ops return `Result<T, RmError>`; `RmError` serializes to `{ kind, message }` (and `status` for `auth`), matching what the UI renders.
- `src-tauri/src/` is the app crate: thin Tauri commands over the manager behind a `tokio::sync::Mutex`. The registered commands are the `generate_handler!` list in `src-tauri/src/lib.rs`.
- Where a tray is available, closing the window hides it and the app can start hidden (`--hidden`, passed by open at login). Without a tray, closing quits.
- Release trains: a stored `Train` (`stable` or `nightly`, optional `keep_last`) makes the app check for the newest release on that train, download it, make it active. Only when `keep_last` is set does it prune: it keeps the newest `keep_last` builds on that train and spares the active one.

## Toolchain prerequisites

- Linux Tauri build: webkit2gtk 4.1 dev, gtk3 dev, librsvg dev, an appindicator dev package and `patchelf`. `Xvfb` is needed for the headless boot smoke.
- Local AppImage bundling on distributions whose libraries use RELR relocations (a `.relr.dyn` section, e.g. recent Fedora): set `NO_STRIP=1` for `tauri build`, because linuxdeploy's bundled `strip` cannot parse it. The CI runner does not need this.
- The AppImage bundles the ubuntu-22.04 webkit and libwayland. On a much newer host the bundled libwayland cannot negotiate EGL with the host compositor, so the window fails to start. `main.rs` preloads the host libwayland and re-execs once (only from the AppImage). The bundled webkit still renders in software on newer GPUs; the real fix (host webkit via rpm/deb) is not done.
- Windows: `winget install Rustlang.Rustup` plus `winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`. Without `--override` winget installs an empty Build Tools shell with no compiler or SDK, and the silent install still raises a UAC prompt to approve on screen. Open a new shell afterwards so `%USERPROFILE%\.cargo\bin` is on PATH.
- The window icon is set at startup as well as bundled. GNOME Wayland ignores it and matches the installed `.desktop` file, so `pnpm dev` shows a generic icon there.

## Tests

- `pnpm test` runs the Rust core tests and vitest. The core tests include a full-flow e2e in `core/tests/e2e_flow.rs` against an in-process fake GitHub server that asserts real filesystem state. This is the primary verification.
- The app crate only compiles and boots with the webkit libraries. A headless boot smoke is `xvfb-run -a` the built debug binary with a throwaway `DA_RELEASE_MANAGER_DIR`.
- There is no WebDriver UI e2e: Fedora ships no `WebKitWebDriver` matching webkit2gtk-4.1, and the backend integration test covers the behavior.
- On Windows two tests differ: the active-folder link test uses a junction (a directory symlink needs Developer Mode), and the config file mode test is unix-only.

## Configuration seams

Read once at startup, no test backdoors: `DA_RELEASE_MANAGER_DIR` (default `~/.da-release-manager`) and `DA_RELEASE_MANAGER_GITHUB_BASE` (default `https://api.github.com`).

```
<dataDir>/
  config.json            # mode 0600; { activeTag?, builds[] }
  cache/<tag>/           # an unzipped chrome build (contains manifest.json)
  cache/.tmp-<tag>/      # transient unzip staging, renamed atomically into place
  active/                # real dir holding a copy of the active build
```

## Active-folder swap (load-bearing invariant)

`set_active(tag)` makes `<dataDir>/active` a real directory and copies the chosen `cache/<tag>/` build into it, replacing its previous contents in place. It is a copy, never a symlink: a sandboxed (Flatpak) Chrome reads the picked folder through a document portal that does not grant the symlink's target, so a symlinked active folder fails with "File path cannot be resolved." Copying keeps the loaded path stable across swaps and works for native and Flatpak Chrome. `remove_build` refuses to delete the active build. A leftover symlink or junction at `active` is removed as a link, never followed; on Windows that takes `remove_dir`, since `remove_file` on a directory link fails with access denied.

## Self-update and signing

- `tauri-plugin-updater` checks `latest.json` on this repo's latest release. The minisign public key is embedded in `tauri.conf.json`; the private key and password are the CI secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Copies older than 0.5.0 poll the `release-manager-latest` release in the extension repo and trust a different key. That feed is frozen at 0.5.0, which carries this repo's key and feed, so old copies move over by updating once. Never delete that release or its tag.

## Release flow

Push a `v<major>.<minor>.<patch>` tag on master. CI stamps the tag's version into `package.json` and `tauri.conf.json` (`scripts/set-version.cjs`), so repo files need no bump. A release tag must point at a commit on master that contains the master-ancestry gate (the workflow runs from the tagged revision, so an older commit has no gate), and the `release-manager-signing` environment now only accepts master and `v*` tags.

The release stays a draft until `scripts/verify-release.sh` passes, and only then is it published as latest, which is when the updater picks it up. If a build or check fails, delete the draft and the tag and tag again. A manual run works only from master (a branch with commits not on master fails the gate). It creates no release; it uploads the AppImage, DMG, installer and portable exe as workflow artifacts. The first install is manual; self-update covers the rest.

`warm-release-cache.yml` compiles the release profile on master and saves it under `shared-key: release-<os>`, because tag runs can only restore caches saved on the default branch. The release jobs restore that key and never save. If the release build's cargo features or the `shared-key` change, change both workflows together.
