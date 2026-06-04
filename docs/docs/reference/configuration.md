# Configuration

## Config directory

`hou` keeps its state in the platform config directory:

| Platform | Location |
| -------- | -------- |
| Linux | `~/.config/hou-cli/` <!-- TODO: verify directory name --> |
| macOS | `~/Library/Application Support/hou-cli/` |
| Windows | `%APPDATA%\hou-cli\` |

## credentials.toml

SideFX API credentials, written by `hou login` and removed by `hou logout`:

```toml
client_id = "..."
client_secret = "..."
```

Used by `hou sidefx`, `hou install`, and anything else that talks to the SideFX API.

## hproject.json

Per-project manifest discovered by walking up from the working directory. Pins the Houdini version and holds the project package manifest.

<!-- TODO: full schema with examples -->

## EULA

Accepted SideFX EULA dates are tracked in settings; manage them with `hou eula`.

## Logging

Set `RUST_LOG` for diagnostic output:

```sh
RUST_LOG=info hou
RUST_LOG=debug hpm install <src>
```
