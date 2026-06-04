# Quickstart

A tour from a fresh install to a running Houdini session.

## 1. Set up

```sh
hou setup
```

## 2. Log in (optional)

Needed for talking to the SideFX API — querying and downloading builds:

```sh
hou login
```

Credentials are stored in `credentials.toml` in the config directory. See [Configuration](../reference/configuration.md).

## 3. Install a Houdini build

```sh
hou install            # install a Houdini build via the discovered installer
hou list               # list installed Houdini products
```

## 4. Launch

```sh
hou                    # latest installed build
hou -v 20.5            # specific version
hou scene.hip          # open a file
hou -- -foreground     # forward args to Houdini after `--`
```

## 5. Create a project

```sh
hou init myproject -v 20.5
cd myproject
hou                    # launches the pinned version with the project environment
```

## 6. Add a package

```sh
hpm install https://github.com/some/houdini-package
hpm list
```

Inside a project the package goes into the project manifest; outside (or with `--global`) it is installed globally.
