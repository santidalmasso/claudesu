<div align="center">
  <img alt="claudesu logo" src="claudesu-symbol.png" height="128">
  <h1>claudesu</h1>
  <p><code>su</code> for Claude Code — switch between accounts without logging out.</p>
</div>

---

Capture each Claude Code account once, then jump between them with a single
command. No logging out, no re-authenticating, no copy-pasting tokens.

## Install

With npm:

```sh
npm install -g claudesu
```

Or with curl:

```sh
curl -fsSL https://github.com/santidalmasso/claudesu/releases/latest/download/install.sh | sh
```

Both install the `csu` command. Prefer not to install? Run it on demand with
`npx claudesu <command>`.

## Quick start

```console
$ csu add                          # capture the account you're signed in as
added alice@acme.com at slot 1

# sign in as your other account in Claude Code, then capture it too:
$ csu add
added alice@personal.com at slot 2

$ csu switch                       # rotate accounts — no login needed
switched to slot 1 (alice@acme.com). Restart Claude Code to pick up the change.

$ csu list
┌───┬────────┬─────────────────────┬──────────────┬────────────┐
│ # ┆ active ┆ email               ┆ organization ┆ added      │
╞═══╪════════╪═════════════════════╪══════════════╪════════════╡
│ 1 ┆ ●      ┆ alice@acme.com      ┆ Acme Inc     ┆ 2026-05-16 │
├╌╌╌┼╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌┤
│ 2 ┆        ┆ alice@personal.com  ┆              ┆ 2026-05-16 │
└───┴────────┴─────────────────────┴──────────────┴────────────┘
```

You only `add` an account once. After that, `switch` swaps them forever.

## Commands

| Command | What it does |
| --- | --- |
| `csu add` | Capture the account Claude Code is currently signed into. |
| `csu list` | List stored accounts; `●` marks the active one. |
| `csu status` | Show the active account. |
| `csu switch` | Rotate to the next account. |
| `csu switch-to <slot\|email>` | Switch to a specific account. |
| `csu remove <slot\|email>` | Forget a stored account. |
| `csu purge` | Forget every account. |

Restart Claude Code after a switch so it picks up the change.

## License

[MIT](LICENSE)
