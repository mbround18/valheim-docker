---
status: "accepted"
date: 2026-09-12
decision-makers: "mbround18, Claude (pairing)"
---

# Install Thunderstore packages using r2modman's BepInEx routes

## Context and Problem Statement

A user reported in [discussion #1516](https://github.com/mbround18/valheim-docker/discussions/1516)
that [ArgusMagnus-ServersideQoL](https://thunderstore.io/c/valheim/p/ArgusMagnus/ServersideQoL/)
fails to load on the server:

```
[Error  :ServersideQoL] ServersideQoL.Patchers.dll was not installed correctly. Put it in /home/steam/valheim/BepInEx/patchers
```

The package ships two top-level folders:

```
patchers/ServersideQoL.Patchers.dll
plugins/ServersideQoL.dll
plugins/ServersideQoL.deps.json
manifest.json, README.md, CHANGELOG.md, icon.png
```

Odin's installer only understood `plugins/`. It moved that folder into
`BepInEx/plugins/<name>/` and then moved _everything else_ into the same place, so the patcher
ended up at `BepInEx/plugins/ServersideQoL/patchers/`. BepInEx only loads preloader patchers from
`BepInEx/patchers`, so the patch never ran and the plugin refused to start. The user's workaround
was copying the DLL by hand after every install.

This is not specific to one mod. Any package that uses `patchers/`, `core/`, `monomod/` or
`config/` was installed in the wrong place. How should odin decide where each part of a package
goes?

## Decision Drivers

- Mods must work on the server the same way they work in a player's mod manager, without manual
  file copying.
- Mod authors already build packages against a known layout; we should read that layout, not
  invent our own.
- Everything odin installs from `MODS` must still be removed when the mod is removed (the
  `from-var-mods.json` cleanup), including files outside `BepInEx/plugins`.
- A server restart re-runs the install. It must not wipe out configuration an admin has edited.
- Existing installs of plugin-only mods must not move or change.

## Considered Options

1. Special-case `patchers/` only.
2. Follow the install rules r2modman and Thunderstore publish for Valheim.
3. Install the package as-is at the `BepInEx/` root (treat top-level folders as literal paths).

## Decision Outcome

Chosen option: **2, follow r2modman's install rules**, because they are the layout Valheim mod
authors actually target, they are published in machine-readable form by Thunderstore, and they
fix the whole class of problem rather than the one reported mod.

### The rules

Thunderstore's ecosystem schema
(`https://thunderstore.io/api/experimental/schema/dev/latest/`, `games.valheim.r2modman`) lists
these install rules for Valheim, and r2modman's
[package structure guide](https://github.com/ebkr/r2modmanPlus/wiki/Structuring-your-Thunderstore-package)
documents the same behaviour:

| Package folder              | Installed to              | Tracking                             |
| --------------------------- | ------------------------- | ------------------------------------ |
| `plugins/` (default)        | `BepInEx/plugins/<mod>/`  | subfolder per mod                    |
| `patchers/`                 | `BepInEx/patchers/<mod>/` | subfolder per mod                    |
| `core/`                     | `BepInEx/core/<mod>/`     | subfolder per mod                    |
| `monomod/`, loose `.mm.dll` | `BepInEx/monomod/<mod>/`  | subfolder per mod                    |
| `SlimVML/`                  | `BepInEx/SlimVML/<mod>/`  | subfolder per mod                    |
| `config/`                   | `BepInEx/config/`         | none (merged into the shared folder) |

Folder names are matched case-insensitively. Anything else (unknown folders, `manifest.json`,
README, loose `.dll` files) goes to `BepInEx/plugins/<mod>/`, which is what odin already did.

### Verified against BepInEx itself

The per-mod subfolder under `patchers/` only works if BepInEx looks inside subfolders. Its docs
only say to [place patchers in `BepInEx/patchers`](https://docs.bepinex.dev/articles/dev_guide/preloader_patchers.html),
so we checked the BepInEx 5 source: `AssemblyPatcher.AddPatchersFromDirectory` calls
`TypeLoader.FindPluginTypes`, which enumerates
`Directory.GetFiles(directory, "*.dll", SearchOption.AllDirectories)`. Subfolders are scanned.

ServersideQoL does not check the file's path either: its error comes from `AssertPatcher()`
failing with `MissingMemberException`, meaning "the patch did not run", so a patcher BepInEx loads
from a subfolder satisfies it.

### Where we deliberately differ from r2modman

- **Subfolder name.** r2modman uses `<Author-ModName>`. Odin keeps using the manifest's `name`
  (`ServersideQoL`), because the package's `manifest.json` has no author field and every existing
  install already lives at `BepInEx/plugins/<name>/`. Changing it would move every installed mod.
- **Config never overwrites.** r2modman merges `config/` and overwrites. On a server that re-runs
  the install on every restart, that would reset admin edits each boot. Odin copies a config file
  only when it does not exist yet, and does not record config files for cleanup: once written,
  they belong to the admin.

### Consequences

- Good: packages with patchers, core libraries or MonoMod patches work without manual steps.
  ServersideQoL installs to `BepInEx/patchers/ServersideQoL/ServersideQoL.Patchers.dll`.
- Good: removing a mod from `MODS` now also removes its `patchers/`, `core/`, `monomod/` and
  `SlimVML/` folders, because each is returned from `install_with_report` and saved in
  `from-var-mods.json`.
- Good: plugin-only packages install exactly where they did before.
- Neutral: a mod's default config lands in `BepInEx/config` on first install, where BepInEx
  expects it, instead of inside the plugin folder where nothing read it.
- Bad: a mod update that changes its default config will not replace an existing file. This is
  the same as BepInEx's own behaviour for generated config and is the safer failure for a server.
- Bad: config left behind after removing a mod has to be deleted by hand.

### Confirmation

- `src/odin/mods/install_routes.rs` unit tests cover each route, case-insensitive folder names,
  unknown folders, re-installs, and that config is merged without overwriting and never tracked.
- `from_var_routes_patchers_and_cleans_them_up` in `src/odin/commands/install_mod.rs` installs a
  ServersideQoL-shaped package through `MODS`, then removes it and checks the patcher and plugin
  folders are gone while the config stays.
- The real `ArgusMagnus-ServersideQoL-2.0.7` package was installed and removed with
  `odin mod:install --from-var` into a scratch game directory.

## Pros and Cons of the Options

### 1. Special-case `patchers/` only

- Good: smallest change.
- Bad: leaves `core/`, `monomod/` and `config/` broken in the same way, so the next report is a
  near copy of this one.
- Bad: a one-off rule with no source to point to when someone asks why.

### 2. Follow r2modman's install rules

- Good: matches how players' mod managers install the same package, which is what authors test.
- Good: the rules come from a published, machine-readable source rather than guesswork.
- Neutral: two intentional differences (subfolder name, config never overwrites) need explaining,
  hence this ADR.

### 3. Install the package as-is at the `BepInEx/` root

- Good: no routing table to maintain.
- Bad: files from different mods would collide in shared folders, and odin could no longer tell
  which files belong to which mod, which breaks cleanup.
- Bad: overwrites admin config on every restart.
- Bad: moves every existing plugin install out of its current `plugins/<name>/` folder.

## More Information

- Thunderstore ecosystem schema: `https://thunderstore.io/api/experimental/schema/dev/latest/`
- [r2modman: Structuring your Thunderstore package](https://github.com/ebkr/r2modmanPlus/wiki/Structuring-your-Thunderstore-package)
- [BepInEx: Using preloader patchers](https://docs.bepinex.dev/articles/dev_guide/preloader_patchers.html)
- BepInEx 5 source: [`AssemblyPatcher.cs`](https://github.com/BepInEx/BepInEx/blob/v5-lts/BepInEx.Preloader/Patching/AssemblyPatcher.cs),
  [`TypeLoader.cs`](https://github.com/BepInEx/BepInEx/blob/v5-lts/BepInEx/Bootstrap/TypeLoader.cs)
- [ServersideQoLPlugin.cs](https://github.com/ArgusMagnus/ValheimServersideQoL/blob/main/ServersideQoL/ServersideQoLPlugin.cs)
