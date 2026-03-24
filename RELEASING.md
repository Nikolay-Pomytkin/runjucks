# Releasing Runjucks

## Secrets (GitHub repository)

| Secret | Used by | Purpose |
|--------|---------|---------|
| `NPM_TOKEN` | [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) | npm automation token with publish permission |
| `CARGO_REGISTRY_TOKEN` | [`.github/workflows/crates-publish.yml`](.github/workflows/crates-publish.yml) | [crates.io API token](https://crates.io/settings/tokens) |

Optional: add `environment: npm` to the `publish-npm` job in [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) and create a GitHub [environment](https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment) named `npm` (e.g. required reviewers or protection rules).

### npm token

1. [npmjs.com](https://www.npmjs.com/) → Access Tokens → **Granular Access Token** (or Classic **Automation**).
2. Scopes: packages **Read and write** for `runjucks` and `runjucks-*` (optional platform packages).
3. Add the token as repository secret **`NPM_TOKEN`**.

### crates.io token

1. [crates.io/settings/tokens](https://crates.io/settings/tokens) → New Token.
2. Add as **`CARGO_REGISTRY_TOKEN`**.

## Version bump (before tagging)

Keep these aligned on the same semver:

1. **Root** [`package.json`](package.json) `version`.
2. **`optionalDependencies`** in `package.json` (must match the root version — same string for each `runjucks-*` entry).
3. [`native/crates/runjucks-core/Cargo.toml`](native/crates/runjucks-core/Cargo.toml) `version`.
4. [`native/crates/runjucks-napi/Cargo.toml`](native/crates/runjucks-napi/Cargo.toml) `version` (not published, but keeps the workspace consistent).

You can bump the npm side with:

```bash
npm version patch   # or minor / major — updates package.json and git-tags locally
```

Then sync `Cargo.toml` versions manually (or script), commit, and tag if `npm version` did not create the tag you want.

Regenerate per-platform `npm/*/package.json` versions if needed:

```bash
npx napi version
```

(Review changes; it updates generated packages under `npm/`, which are not committed — CI recreates them.)

## Publish flow

1. Push a commit with bumped versions.
2. Create a **GitHub Release** from the tag and **publish** it (not draft).
3. **Publish npm** and **Publish crates.io** workflows run in parallel.

### Dry run npm locally (optional)

```bash
npm ci
npm run build
npx napi create-npm-dirs
mkdir -p artifacts && cp runjucks.*.node artifacts/
npm run napi:artifacts
npm publish --dry-run
# Platform packages live under npm/* — napi pre-publish normally publishes them; use CI for real publishes.
```

## crates.io notes

- Only **`runjucks_core`** is published; **`runjucks-napi`** has `publish = false` (Node binary crate).
- First publish must use an owner account that accepts the new crate name.
- If `cargo publish` complains about missing fields, check `runjucks-core/Cargo.toml` metadata.

## npm notes

- **Provenance** is enabled via `publishConfig.provenance` and `npm config set provenance true` in CI (requires `id-token: write` — already set in the workflow).
- If provenance fails for your org, remove `"provenance": true` from `package.json` and the `npm config set provenance true` line in the workflow.
