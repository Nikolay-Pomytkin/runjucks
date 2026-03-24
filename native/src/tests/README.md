# Integration tests

| File | Focus |
|------|--------|
| `environment.rs` | `Environment::render_string` |
| `interpolation.rs` | `{{ }}` variables |
| `lexer.rs` | Tokenizer, `{# #}` |
| Other `*.rs` | Parser, renderer, filters, … |

```bash
# from the npm package root (parent of `native/`)
cargo test --manifest-path native/Cargo.toml
# or
npm run test:rust
```

To run a subset without `interpolation.rs`, use `npm run test:rust:green` from the package root.
