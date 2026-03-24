# Integration tests

| File | Focus |
|------|--------|
| `environment.rs` | `Environment::render_string` |
| `interpolation.rs` | `{{ }}` variables |
| `lexer.rs` | Tokenizer, `{# #}`, `{% %}` (pending) |
| `lexer_control_flow.rs` | Nunjucks block tags as `Token::Tag` bodies |
| `lexer_whitespace.rs` | `{%-`, `-%}`, `{{-`, `-}}` |
| `parser.rs` | Basic parse |
| `parser_expressions.rs` | Literals, operators (parity with nunjucks parser tests) |
| `parser_tags.rs` | `{% %}` tokenization for control flow |
| `conformance.rs` | JSON goldens in [`../../fixtures/conformance/`](../../fixtures/conformance/README.md) |
| Other `*.rs` | Renderer, filters, value, … |

```bash
# from the npm package root (parent of `native/`)
cargo test --manifest-path native/Cargo.toml
# or
npm run test:rust
```

- **`npm run test:rust:green`** — subset that excludes long-running / parity crates (see `package.json`).
- **`npm run test:conformance:rust`** / **`npm run test:conformance:node`** — shared JSON fixtures (Rust vs `renderString`).
