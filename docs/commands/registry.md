# `mnem registry`

A local file-based registry: a folder of exported agent bundles, one
JSON file each.

## Synopsis

```
mnem registry publish [OPTIONS] <NAME>
mnem registry list [OPTIONS]
mnem registry fork [OPTIONS] <NAME> <DEST>
mnem registry render [OPTIONS] --output <FILE>
```

## Description

Four thin wrappers over [`export`](export.md)/[`import`](import.md),
no new object kind, no network: `publish` exports the current store
to `<registry>/<name>.json`; `list` reads the folder; `fork` reads one
bundle back and imports it into a new store; `render` writes every
bundle in the folder as one static HTML page (name, prompt summary,
tool count). The registry folder is always explicit - `--registry
<DIR>` on every subcommand, or `$MNEM_REGISTRY` as a fallback, the
same shape `--author`/`$MNEM_AUTHOR` already uses - there is no
invented default location mnem writes to on its own.

`render` reads each bundle's `agent:prompt`/`agent:tools` fields,
which are a worked-example convention
([`agent_bundle_export_import.py`](https://github.com/Nabzx/mnemosyne/blob/main/examples/agent_bundle_export_import.py)),
not a real node-id standard - a bundle missing either field just
renders `(none)`.

## Options

| Subcommand | Argument / Flag | Description |
| --- | --- | --- |
| all | `--registry <DIR>` | The registry folder. Falls back to `$MNEM_REGISTRY`. |
| `publish` | `<NAME>` | The bundle's name in the registry. |
| `fork` | `<NAME>` | The bundle's name in the registry. |
| `fork` | `<DEST>` | Where to create the forked store. Refuses if one already exists there. |
| `render` | `--output <FILE>` | Write the page here. |

## Examples

```
$ mnem registry publish support-agent --registry ./registry
Published support-agent (4 object(s)) to ./registry/support-agent.json
$ mnem registry list --registry ./registry
support-agent
$ mnem registry fork support-agent ./clone --registry ./registry
Forked support-agent into ./clone/.mnem (4 object(s))
$ mnem registry render --registry ./registry --output ./registry/index.html
Wrote the registry page (1 bundle(s)) to ./registry/index.html
```

## See also

- [`export`](export.md), [`import`](import.md) - what every
  `registry` subcommand is built from.
