# Running Tools

`hou run` (alias: `houx`) runs any command inside the resolved Houdini build's environment — same version resolution and project discovery as launching Houdini itself.

```sh
hou run hython script.py     # run hython
houx hbatch -c "render" scene.hip
houx hserver --status
```

## houpy

`houpy` is shorthand for `hou run hython`:

```sh
houpy script.py
houpy -c "import hou; print(hou.applicationVersionString())"
```

## Version resolution

- Inside a project, the project's pinned version is used (`--version` is ignored).
- Outside a project, `--version` filters installed builds; otherwise the latest is used.
