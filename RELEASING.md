# Releasing

Releases are cut by pushing a `v*` tag. The [`release` workflow](.github/workflows/release.yml)
then builds the binaries, publishes the GitHub release, and publishes the npm package.

## One-time setup

1. Create the GitHub repo `santidalmasso/claudesu` and push this code to it.
2. Add an `NPM_TOKEN` repository secret (Settings → Secrets and variables →
   Actions). Use an npm **automation** token for an account that may publish the
   `claudesu` package.

## Cutting a release

1. Bump `version` in `Cargo.toml`, then run `cargo build` to refresh `Cargo.lock`.
2. Commit the change: `git commit -am "release vX.Y.Z"`.
3. Tag and push:
   ```sh
   git tag vX.Y.Z
   git push origin main --tags
   ```

The tag's version must match `Cargo.toml` — the workflow fails fast otherwise.
The npm package version is set from the tag automatically, so it does not need
to be edited by hand.

## What the workflow produces

For each release the workflow attaches these assets:

- `csu-<target>` / `csu-<target>.exe` — the prebuilt binary per platform
- `csu-<target>.sha256` — its checksum
- `install.sh` — the curl installer

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-pc-windows-msvc`.

## Install methods these enable

- `curl -fsSL https://github.com/santidalmasso/claudesu/releases/latest/download/install.sh | sh`
- `npm install -g claudesu`
