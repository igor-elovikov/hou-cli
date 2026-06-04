# CLI Reference

## hou (default launch)

```
hou [OPTIONS] [FILE] [-- <HOUDINI_ARGS>...]
```

| Option | Description |
| ------ | ----------- |
| `FILE` | Optional file (e.g. a `.hip` file) or project directory to open |
| `-v, --version <VERSION>` | Houdini version filter (ignored inside a project) |
| `-a, --attach` | Keep stdio attached to the terminal and wait for Houdini to exit |
| `-- <ARGS>...` | Arguments forwarded to Houdini |

## hou setup

First-time setup: discover installations and the SideFX launcher.

## hou update

Update the SideFX launcher to the latest production build.

## hou init

```
hou init [NAME] [-v <VERSION>]
```

Initialize a Houdini project — in `NAME` if given, otherwise the current directory. `--version` pins the Houdini version in the project options.

## hou run (`houx`)

```
hou run [-v <VERSION>] <COMMAND> [ARGS]...
```

Run a command in the resolved Houdini build's environment. `houpy` = `hou run hython`.

## hou package (`hpm`)

```
hou package [OPTIONS] <ACTION>
```

| Option | Description |
| ------ | ----------- |
| `--global` | Operate on the global manifest, even inside a project |
| `--project` | Operate on the project manifest (requires being inside a project) |
| `-v, --version <VERSION>` | Houdini version filter (inside a project requires `--global`) |
| `--no-patch` | Skip patching package json files after install/update/sync |

Actions:

| Action | Description |
| ------ | ----------- |
| `install <SOURCE>` | Install a package from a URL, local git repo, or folder (`--name`, `--tag`, `--head`) |
| `uninstall <NAME>` | Remove a package by name or install path |
| `update <NAME>` | Update a git package to a new version |
| `list` | List installed packages |
| `sync` | Re-fetch any git package whose cache dir is missing or has a checksum mismatch |

## hou install / uninstall / list

Manage installed Houdini builds via the discovered installer.

## hou sidefx

```
hou sidefx builds <PRODUCT> [-v <VERSION>] [-p <PLATFORM>] [--all]
hou sidefx download
```

Query the SideFX API. `builds` lists production builds by default; `--all` includes everything.

## hou login / logout

Store or remove SideFX credentials (`credentials.toml` in the config directory).

## hou eula

Manage accepted SideFX EULA dates.

<!-- TODO: generate this page from `clap` definitions to keep it in sync -->
