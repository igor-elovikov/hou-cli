# Packages

`hou package` (alias: `hpm`) manages Houdini packages — from a URL, a git repository, or a local folder.

```sh
hpm install <url|git-repo|folder>    # install a package
hpm install <src> --name custom      # override the install-directory name
hpm install <src> --tag v1.2.0       # pin a specific tag (or raw commit)
hpm install <src> --head             # track HEAD instead of a tag
hpm uninstall <name|path>            # remove a package
hpm update <name>                    # update a git package to a new version
hpm list                             # list installed packages
hpm sync                             # re-fetch git packages with a missing/mismatched cache
```

## Project vs. global scope

- **Inside a project**, packages operate on the project manifest by default.
- `--global` targets the global manifest even when inside a project.
- `--project` requires being inside a project.
- `--version <v>` filters the Houdini version; inside a project it requires `--global`.

## Patching

After install/update/sync, package json files are patched for the target environment. Skip this with `--no-patch`.

<!-- TODO: document what patching rewrites and why -->

## Caching & checksums

Git packages are cached locally and verified by checksum; `hpm sync` re-fetches any package whose cache dir is missing or fails verification.

<!-- TODO: cache location, layout, manual cache management -->
