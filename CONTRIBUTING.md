# Contributing to Runjucks

## Package managers: Node first, Bun optional

**Canonical workflow (matches CI):** **npm** + **Node.js**. The addon targets **Node ≥ 18** (see root `package.json` `engines`). Install and run scripts from the `runjucks/` directory:

```bash
npm ci          # or npm install
npm run build
npm test
npm run docs:dev
```

**Optional [Bun](https://bun.sh):** you may use `bun install` and `bun run <script>` for the same script names defined in `package.json`. **Lockfiles** (`package-lock.json`, `docs/package-lock.json`) stay npm-oriented; do not replace them unless the project explicitly adopts another lockfile.

**Documentation site** (`docs/`): follow **Astro’s** Node requirement in `docs/package.json` `engines` (currently **≥ 22.12**). From repo root, root `docs:*` scripts use `npm --prefix docs`. With Bun locally: `cd docs && bun run dev` (or `bun run build`, etc.).

**Caveats:**

- **Native NAPI** builds are exercised on **Node** in GitHub Actions. If something fails under Bun, use Node/npm.
- **`npm test`** runs **`node --test`**. Do not assume **`bun test`** is equivalent; use `npm test` or **`bun run test`** (runs the npm script, which invokes `node --test`).

## Rust and Node

- Rust layout: workspace at `native/Cargo.toml` — engine crate **`runjucks_core`**, NAPI crate **`runjucks-napi`** (`napi build` uses the latter).
- Rust integration tests: `npm run test:rust` or `cargo test --manifest-path native/Cargo.toml`
- See the main [README](README.md) for layout, architecture, and full command list.
