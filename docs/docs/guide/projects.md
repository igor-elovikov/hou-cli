# Projects

A Houdini project is a directory containing an `hproject.json` manifest. Projects pin a Houdini version and scope packages and environment to the project.

## Creating a project

```sh
hou init                 # initialize the current directory
hou init myproject       # create and initialize ./myproject
hou init myproject -v 20.5
```

`init` scaffolds a standard Houdini package layout:

```
otls/
scripts/python/
config/Icons/
toolbar/
vex/include/
ocl/include/
viewer_states/
viewer_handles/
desktop/
python_panels/
```

## hproject.json

<!-- TODO: document the full schema -->

Key fields:

- `houdini_version` — the version pin used by `hou`, `hou run`, and `hou package`.

## Version pinning

Inside a project the pinned version always wins; passing `--version` prints a warning and uses the project pin instead. The exception is `hou package --global --version <v>`, which escapes the pin to operate on the global manifest for a specific build.
