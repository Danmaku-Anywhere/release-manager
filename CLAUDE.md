# CLAUDE.md

Danmaku Builds (this repo, formerly Release Manager) is a Tauri 2 app that keeps one Danmaku Anywhere extension build as the unpacked folder loaded into Chrome. Architecture, toolchain, tests and the release flow are in `AGENTS.md`.

- Use mise for Node (pinned in `mise.toml`) and rustup for Rust.
- PR titles: `(app) description [DA-123]`, with the ClickUp task id. The `PR Title Check` workflow enforces it.
- Comments only for a hidden constraint or a trap. No em dashes in code, comments, commits or PR text.
- Never commit signing keys. The updater key lives in the `release-manager-signing` environment and in Proton Pass.
