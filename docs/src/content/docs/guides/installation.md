---
title: Installation
description: Add Runjucks to a Node.js project and build the native addon.
---

## Prerequisites

- **Node.js** — **≥ 18** for the **published npm package** and native addon builds (see root `package.json` `engines`). The **documentation site** in `docs/` is built with **Astro** and requires **Node ≥ 22.12** (see `docs/package.json` `engines`); use that version when running `npm run dev` or `npm run build` in `docs/`.
- **Rust** (stable) and a C toolchain (for `napi-rs` native builds)

## From the repository

```bash
cd runjucks
npm install
npm run build
```

`npm run build` runs `napi build` and produces `runjucks.*.node`, `index.js`, and `index.d.ts` for your platform.

Debug build:

```bash
npm run build:debug
```

## Using the package

```js
import { renderString, Environment } from '@zneep/runjucks'

console.log(renderString('Hello {{ name }}', { name: 'Ada' }))

const env = new Environment()
env.setAutoescape(true)
console.log(env.renderString('Plain text', {}))
```

See the [Node.js API reference](../../api/) for `Environment` methods.
