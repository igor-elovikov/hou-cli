# Managing Builds

Install, remove, and query Houdini builds.

## Local builds

```sh
hou list                 # list installed Houdini products
hou install              # install a build via the discovered installer
hou uninstall            # uninstall an installed build
hou update               # update the SideFX launcher to the latest production build
```

## Querying SideFX

Requires SideFX API credentials — see [`hou login`](../reference/configuration.md).

```sh
hou sidefx builds houdini                 # production builds
hou sidefx builds houdini --all           # include daily builds
hou sidefx builds houdini -v 20.5         # filter by version
hou sidefx builds houdini -p linux        # filter by platform
hou sidefx download                       # download a build
```

<!-- TODO: document supported products and platforms; flesh out download flow -->

## EULA

```sh
hou eula                 # manage accepted SideFX EULA dates
```
