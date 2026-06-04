# hou

A command-line companion for SideFX Houdini: launch any installed build, manage per-project Houdini versions, and install packages — all from the terminal.

## What it does

- **Launch Houdini** with a single command, picking the right build automatically — or pinned per project.
- **Projects** — initialize a Houdini project with `hou init`, pin its Houdini version in `hproject.json`, and have everything (launch, packages, tools) respect it.
- **Packages** — install Houdini packages from a URL, git repo, or local folder; update, list, and sync them. Available as the `hpm` alias.
- **Builds** — install and uninstall Houdini builds, query SideFX for available builds, and keep the launcher up to date.
- **Tools** — run Houdini binaries (`hython`, `hbatch`, …) in the resolved build's environment via `hou run` / `houx` / `houpy`.

## Quick look

```sh
hou                      # launch Houdini from the current directory
hou init myproject       # create a new Houdini project
hpm install <git-url>    # install a package
houpy script.py          # run a script with hython
```

## Where to go next

- [Installation](getting-started/installation.md) — get the `hou` binary and its aliases.
- [Quickstart](getting-started/quickstart.md) — from zero to a running Houdini session.
- [CLI Reference](reference/cli.md) — every command and flag.
