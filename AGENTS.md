# AGENTS.md

## Cursor Cloud specific instructions

Runjucks is a Nunjucks-compatible template engine with a Rust core exposed to Node.js via NAPI-RS. It is a library, not a web application — there are no long-running services, databases, or Docker dependencies.

### Prerequisites

- **Rust stable** (≥ 1.88, for napi-rs dependency compatibility). The VM's default toolchain may be older; run `rustup update stable && rustup default stable` if `npm run build` fails with version errors.
- **Node.js ≥ 18** and **npm** (lockfile is `package-lock.json`).

### Key commands

All commands run from the repo root (`/workspace`). See `README.md` and `package.json` `scripts` for the full list.

| Task | Command |
|------|---------|
| Install deps | `npm install` |
| Build native addon | `npm run build` |
| Build (debug) | `npm run build:debug` |
| Node tests | `npm test` |
| Rust tests | `npm run test:rust` |
| Rust lint | `cargo clippy --manifest-path native/Cargo.toml --all-targets` |
| Perf benchmarks | `npm run perf` |

### Gotchas

- **Build before test:** `npm test` requires the `.node` native addon to exist. Always run `npm run build` (or `npm run build:debug` for faster compile) before `npm test`.
- **No JS lint script:** The project has no ESLint config or `npm run lint`. Lint checks are Rust-only via `cargo clippy`. The codebase uses `#![deny(clippy::all)]`, so clippy warnings are treated as errors.
- **Clippy may report pre-existing issues:** As of writing, `cargo clippy` reports existing lint issues in the codebase that do not affect build or tests. This is expected and does not block development.
- **Optional dependencies:** `package.json` lists platform-specific optional deps (`@zneep/runjucks-*`) that will fail to install on non-matching platforms; this is normal and handled by npm's optional dependency resolution.
