# Releasing Runjucks

## Secrets (GitHub repository)

| Secret | Used by | Purpose |
|--------|---------|---------|
| `NPM_TOKEN` | [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) | npm automation token with publish permission |
| `CARGO_REGISTRY_TOKEN` | [`.github/workflows/crates-publish.yml`](.github/workflows/crates-publish.yml) | [crates.io API token](https://crates.io/settings/tokens) |

Optional: add `environment: npm` to the `publish-npm` job in [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) and create a GitHub [environment](https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment) named `npm` (e.g. required reviewers or protection rules).

### npm token

**You do not need packages to exist on npm first.** The first successful `npm publish` **creates** each package under your npm user (or org). The confusing part is only which **kind** of token to use.

#### First release — use a Classic Automation token (simplest)

If you have **no packages** on the registry yet (or you are not sure), skip granular tokens for CI.

1. Sign in at [npmjs.com](https://www.npmjs.com/).
2. Profile avatar → **Access Tokens** → **Generate New Token** → **Classic Token**.
3. Type **Automation** — intended for CI; it can publish **new** package names your account is allowed to register ([npm docs](https://docs.npmjs.com/creating-and-viewing-access-tokens)).
4. Copy the token once and set GitHub secret **`NPM_TOKEN`** to that value.

When the workflow runs `npm run napi:prepublish`, it will create **`@zneep/runjucks`** plus each **`@zneep/runjucks-*`** optional native package the first time they publish.

**Why scoped?** npm rejects the unscoped name `runjucks` as too similar to the existing `nunjucks` package (typosquatting protection). The scoped name is allowed.

#### Later — granular token (tighter scope)

After those names exist on npm, you can replace **`NPM_TOKEN`** with a **Granular Access Token** if you want least privilege. Grant **Read and write** on **`@zneep/runjucks`** **and** every name in [`package.json`](package.json) `optionalDependencies`:

- `@zneep/runjucks-darwin-arm64`
- `@zneep/runjucks-darwin-x64`
- `@zneep/runjucks-linux-arm64-gnu`
- `@zneep/runjucks-linux-arm64-musl`
- `@zneep/runjucks-linux-x64-gnu`
- `@zneep/runjucks-linux-x64-musl`
- `@zneep/runjucks-win32-x64-msvc`

Granular tokens do **not** treat `@zneep/runjucks-*` as a wildcard. If you only add **`@zneep/runjucks`**, CI will hit **`403 Forbidden — You may not perform that action with these credentials`** when it tries to publish the first platform package.

**Organizations:** if packages live under an npm org, the token’s user must be a member with publish rights (or use an org-scoped automation setup per npm’s docs).

**Local `npm publish` fails with 403:**  
`Two-factor authentication or granular access token with bypass 2fa enabled is required to publish packages.`  
Your account requires 2FA for publishing; a normal **`npm login`** token is often not enough. Create a **Classic** token with type **Automation**, then:

```bash
export NPM_TOKEN='…'
npm config set //registry.npmjs.org/:_authToken="${NPM_TOKEN}"
npm run napi:prepublish
```

Do not use a granular token unless it explicitly allows automation / bypass 2FA for publish.

### crates.io token

1. [crates.io/settings/tokens](https://crates.io/settings/tokens) → New Token.
2. Add as **`CARGO_REGISTRY_TOKEN`**.

## Documentation: performance report on the site

The [Performance](docs/src/content/docs/guides/performance.mdx) guide embeds a **snapshot** of `npm run perf:json` (vs the `nunjucks` version in root `devDependencies`). Reports are **committed** under [`docs/src/data/perf/reports/`](docs/src/data/perf/reports/) so the published site shows numbers tied to **`@zneep/runjucks` version** without running benches on every CI build.

After a release version bump (or when you intentionally refresh benchmarks):

1. From the **package root**: `npm run build && npm run perf:json` (uses **warm** Runjucks parse cache by default; use `npm run perf:cold` if you want a cold snapshot—then save under a distinct filename or document `mode` in the report).
2. Copy `perf/last-run.json` to `docs/src/data/perf/reports/<runjucksVersion>.json` (for example `0.1.8.json`).
3. Update [`docs/src/data/perf/index.json`](docs/src/data/perf/index.json): set **`latest`** to that version and append it to **`reports`** if it is a new file (keep older versions in **`reports`** for optional history).

Numbers are **machine-dependent**; treat them as directional. The JSON records **`platform`**, **`node`**, and timestamps so readers can see how the snapshot was produced.

## Version bump

Keep these aligned on the same semver in **one commit** on `main` (so both registries release the same release):

1. **Root** [`package.json`](package.json) `version`.
2. **`optionalDependencies`** in `package.json` (must match the root version — same string for each `@zneep/runjucks-*` entry).
3. [`native/crates/runjucks-core/Cargo.toml`](native/crates/runjucks-core/Cargo.toml) `version`.
4. [`native/crates/runjucks-napi/Cargo.toml`](native/crates/runjucks-napi/Cargo.toml) `version` (not published, but keeps the workspace consistent).

You can bump the npm side with:

```bash
npm version patch   # or minor / major — updates package.json (and optional git tag)
```

Then sync `Cargo.toml` versions manually (or script) and commit. **Tags are optional** for publishing; workflows trigger from `main`.

Regenerate per-platform `npm/*/package.json` versions if needed:

```bash
npx napi version
```

(Review changes; it updates generated packages under `npm/`, which are not committed — CI recreates them.)

## Publish locally (manual, before CI automation)

You can run the same publish step as CI on your machine after **`npm login`**. You do **not** need packages to exist on npm yet; the first `npm publish` creates each name under your account.

**Important:** The npm package ships **one native addon per OS/CPU** (Windows, macOS Intel, macOS ARM, Linux gnu/musl × x64/ARM). That is **seven** different `runjucks.*.node` files. Your laptop can only build **one** of them per `npm run build` unless you set up cross-compilation like CI does.

To publish **all** platforms from your machine, you need **all seven `.node` files** on disk first. The usual way is to **reuse GitHub Actions as a build farm**, then pull the built files down:

1. Push/trigger a **`Publish npm`** workflow run where the **`build-native`** job finishes successfully (it is OK if **`publish-npm`** fails—e.g. bad token—you still get the binaries).
2. In the browser: **GitHub → your repo → Actions → open that workflow run → scroll to “Artifacts”** at the bottom. You will see several downloads named like `bindings-x86_64-pc-windows-msvc`, `bindings-aarch64-apple-darwin`, etc. Those are **not** in the git repo; GitHub only stores them for that run.
3. Download **each** artifact. GitHub delivers each one as a **`.zip`** (that is the only “zip” involved—your browser’s download). Unzip them (double-click on macOS or `unzip` in a terminal). Inside each is a **`runjucks.<platform>.node`** file.
4. Put all those `.node` files in **one folder** on your machine (e.g. `~/Downloads/runjucks-bindings/`). You should end up with **seven** files before continuing.

If you **skip GitHub entirely**, you only have whatever `npm run build` just wrote in the **`runjucks/`** folder—**one** platform. You cannot complete a multi-platform npm release that way without cross-builds or more machines.

Then from the **`runjucks/`** package root (this folder in the repo):

```bash
npm ci
npx napi create-npm-dirs
mkdir -p artifacts
# Folder from step 4 above (must actually exist on your machine):
BINDINGS_DIR="$HOME/Downloads/runjucks-bindings"
find "$BINDINGS_DIR" -name 'runjucks.*.node' -exec cp -t artifacts/ {} +
npm run napi:artifacts
npm login
# Locally, OIDC provenance (CI) does not apply. Avoid provenance errors:
npm config set provenance false
npm run napi:prepublish
```

Restore `provenance` expectations before relying on CI again (CI runs `npm config set provenance true` in the workflow).

**crates.io (optional, same release):**

```bash
cd native/crates/runjucks-core
cargo login   # paste crates.io API token
cargo publish
```

## Publish flow (automatic)

1. Merge or push to **`main`** with the version fields above updated.
2. **[Publish npm](.github/workflows/npm-publish.yml)** runs when `package.json` / `package-lock.json` change. It only publishes if the **`version`** in `package.json` is **higher** than on the parent commit **and** that version is **not** already on npm (so dependency-only edits to `package.json` do not publish).
3. **[Publish crates.io](.github/workflows/crates-publish.yml)** runs when `native/crates/runjucks-core/Cargo.toml` changes. It only publishes if the crate **`version`** increased versus the parent commit **and** that version is not already on crates.io.
4. After a successful npm publish, the same workflow creates a **GitHub Release** named `v<version>` on the triggering commit. Notes list **non-merge commits** since the previous `v*` tag (version-sorted), or the last 500 commits if no tag exists yet. If the tag already exists, the step **skips** (safe reruns). If npm succeeded but this step failed, create the release manually or fix permissions; rerunning the workflow will usually **not** redo the release when npm skips as “already published.”

If you bump **only** `package.json` or **only** `runjucks-core/Cargo.toml`, only the matching workflow will try to publish — keep versions aligned and touch both files in the same commit for a coordinated release.

### Manual retry

In **Actions**, open **Publish npm** or **Publish crates.io** → **Run workflow**. The job still skips if that version already exists on the registry (to avoid duplicate publishes).

Workflows listen to the **`main`** branch. If your default branch is different, rename it in [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) and [`.github/workflows/crates-publish.yml`](.github/workflows/crates-publish.yml).

### Dry run npm locally (optional)

```bash
npm ci
npm run build
npx napi create-npm-dirs
mkdir -p artifacts
# After build, the .node file is in this directory; use find so zsh won't fail if the glob matches nothing:
find . -maxdepth 1 -name 'runjucks.*.node' -exec cp -t artifacts/ {} +
npm run napi:artifacts
npm publish --dry-run
# Platform packages live under npm/* — `napi:prepublish` uses `--tag-style npm` (not Lerna-style commit lines) and `--no-gh-release` so CI does not depend on GitHub Release commit format or upload assets to a release.
```

## crates.io notes

- Only **`runjucks_core`** is published; **`runjucks-napi`** has `publish = false` (Node binary crate).
- First publish must use an owner account that accepts the new crate name.
- If `cargo publish` complains about missing fields, check `runjucks-core/Cargo.toml` metadata.

## npm notes

- **Provenance** (supply-chain attestations in CI) is turned on in [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml) with `npm config set provenance true` (requires `id-token: write` — already set). **`publishConfig.provenance` is intentionally not set** in `package.json`: if it were `true`, local `npm publish` would fail with `Automatic provenance generation not supported for provider: null` because only GitHub Actions OIDC provides a provider.
- **Local** `npm publish` / `napi:prepublish` works without extra flags once `provenance` is not in `publishConfig`.
- If provenance in CI breaks for your org, drop `npm config set provenance true` from the workflow.
