---
title: Package managers
description: Node/npm as the default toolchain; optional Bun for local development.
---

The project is **Node-first**: **npm** and the Node versions in `package.json` / `docs/package.json` `engines` are what **CI** uses.

## Default (npm)

From the repository root (`runjucks/`):

```bash
npm install
npm run build
npm test
npm run docs:dev
```

## Optional Bun

You may use **Bun** with the same script names (`bun install`, `bun run build`, …). For the docs app only, from `docs/`: `bun run dev`, `bun run build`, etc.

## Caveats

- The **native addon** is validated on **Node** in CI; if Bun misbehaves, switch to Node.
- **`bun test`** is not the same as **`npm test`** (which uses `node --test`). Prefer `npm test` or `bun run test` so you run the project script.

For a fuller contributor-oriented summary, see **`CONTRIBUTING.md`** at the repository root (sibling of this `docs/` folder).
