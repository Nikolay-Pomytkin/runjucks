---
title: Rust API (rustdoc)
description: How to browse the internal Rust crate documentation.
---

The npm package is a thin NAPI layer over the `runjucks` crate in `native/`. Most users only need the [Node.js API](../../api/) (`renderString`, `Environment`).

## Local rustdoc

From the repository root (`runjucks/`):

```bash
cargo doc --manifest-path native/Cargo.toml --no-deps --open
```

This opens HTML docs for the crate and its public items. Modules that are `#[doc(hidden)]` in `lib.rs` are omitted from the public API surface but you can still read them in-tree.

## docs.rs

If the crate is published to [crates.io](https://crates.io), documentation will appear at `https://docs.rs/runjucks` automatically. Until then, use local `cargo doc` as above.
