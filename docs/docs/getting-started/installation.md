# Installation

<!-- TODO: link release artifacts / install script once published -->

## From releases

Download the latest release for your platform from the [releases page](https://github.com/igor-elovikov/hou-cli/releases).

## From source

```sh
cargo install --git https://github.com/igor-elovikov/hou-cli
```

## Bin aliases

The single `hou` binary dispatches on its invocation name, so the following aliases are available alongside it:

| Alias   | Equivalent           | Purpose                              |
| ------- | -------------------- | ------------------------------------ |
| `hpm`   | `hou package`        | Package manager                      |
| `houx`  | `hou run`            | Run a tool in the Houdini environment |
| `houpy` | `hou run hython`     | Run hython directly                  |

Release archives ship these as separate launchers; if you build from source, create symlinks (or copies) named `hpm`, `houx`, and `houpy` pointing at the `hou` binary.

## First-time setup

```sh
hou setup
```

Discovers your Houdini installations and the SideFX launcher. See the [Quickstart](quickstart.md) for the full walkthrough.
