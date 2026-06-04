# Launching Houdini

Running `hou` with no subcommand launches Houdini.

```sh
hou                      # launch from the current directory
hou scene.hip            # open a .hip file
hou path/to/project      # launch inside a project directory
hou -v 21.0              # pick a build by version filter
hou -a                   # --attach: keep stdio attached, wait for exit
hou -- -foreground       # everything after `--` is forwarded to Houdini
```

## Version resolution

- **Inside a project** the version pinned in `hproject.json` wins; a `--version` flag is ignored with a warning.
- **Outside a project** `--version` filters the installed builds; without it the latest build is used.

## Project discovery

`hou` walks up from the current directory (or from the file/directory argument) looking for `hproject.json`. If found, the project's environment and packages are applied to the session. See [Projects](projects.md).

<!-- TODO: document environment variables set for the session -->
