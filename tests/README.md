# Integration tests

| File | Focus |
|------|--------|
| `environment.rs` | `Environment::render_string` |
| `interpolation.rs` | `{{ }}` variables |
| `lexer.rs` | Tokenizer, `{# #}` |
| Other `*.rs` | Parser, renderer, filters, … |

```bash
cargo test
```

To run a subset without `tests/interpolation.rs`, use `npm run test:rust:green` from the package root.
