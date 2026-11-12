# CLAUDE.md

Danmaku Builds (this repo, formerly Release Manager) is a Tauri 2 desktop app that downloads Danmaku Anywhere
extension builds from `Danmaku-Anywhere/danmaku-anywhere` releases and keeps one
of them as the unpacked folder loaded into Chrome. Architecture, toolchain,
tests and the release flow are in `AGENTS.md`.

## Commands

| Task       | Command                       |
| ---------- | ----------------------------- |
| Install    | `pnpm install`                |
| Dev        | `pnpm dev`                    |
| Type check | `pnpm type-check`             |
| Lint       | `pnpm lint`                   |
| Test       | `pnpm test` (Rust core tests) |

Use mise for Node and pnpm, and rustup for Rust.

## Conventions

- Biome for formatting and linting (`biome.json`), single quotes, no semicolons.
- PR titles: `(app) description [DA-123]`, with the ClickUp task id. The
  `PR Title Check` workflow enforces it.
- Default to no comments; add one only for a hidden constraint or a trap.
- No em dashes in code, comments, commits or PR text.
- Never commit signing keys. The updater key lives in the
  `release-manager-signing` environment and in Proton Pass.
