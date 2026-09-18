# Gale Profile Sync

[Gale](https://github.com/Kesomannen/gale) can publish a mod profile and hand out a short
sync code ([Gale wiki: Profile sync](https://github.com/Kesomannen/gale/wiki/Profile-sync)).
Players pull the code to get the exact same mods. Give the server that code too and it installs
the same mods at the same versions, so nobody gets an "incompatible mod versions" error after
the profile owner updates something.

## Quick start

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    environment:
      - TYPE=BepInEx
      - GALE_SYNC_CODE=abc123
```

On every start, Odin reads the profile from Gale and installs each **enabled** mod at the
version the profile pins:

- Mods from Thunderstore become `ts:Author-Mod-1.2.3` entries, mods from Hexium become
  `hex:Author-Mod-1.2.3` (see [Mod Repositories](./mod_repositories.md)).
- Disabled mods are skipped.
- `BepInExPack_Valheim` is skipped because the container installs BepInEx itself.

When the owner pushes a new version of the profile, restart the server to pick it up. Mods the
owner removed are uninstalled, just like removing them from `MODS`.

## Adding server-only mods

`MODS` still works next to `GALE_SYNC_CODE`. Its entries are installed after the profile's, so
you can add mods that only the server needs:

```yaml
environment:
  - TYPE=BepInEx
  - GALE_SYNC_CODE=abc123
  - |
    MODS=ts:Azumatt-AzuAntiCheat-4.3.11
    https://example.com/MyServerPlugin.dll
```

If a `MODS` entry names a mod that is also in the profile, the `MODS` entry wins. Use this to
pin a different version without changing the profile.

## Syncing configs

The profile also carries the owner's `BepInEx/config` files. Set `GALE_SYNC_CONFIGS=true` to
copy them over the server's on every start:

```yaml
environment:
  - TYPE=BepInEx
  - GALE_SYNC_CODE=abc123
  - GALE_SYNC_CONFIGS=true
```

This overwrites any file with the same name, so edits you make on the server to those files
are replaced at the next start. Config files that are not in the profile are left alone. If
the configs cannot be downloaded, Odin logs a warning and starts with the configs it has.

## When Gale is unreachable

Odin keeps the last profile it fetched. If Gale is down, it installs from that copy and logs
`using the last synced copy`. On the very first start there is no copy yet, so an unreachable
Gale fails the mod install like any other failed download.

## Settings

| Variable            | Default                           | Description                                                                 |
| ------------------- | --------------------------------- | --------------------------------------------------------------------------- |
| `GALE_SYNC_CODE`    | `<unset>`                         | The profile's sync code. Requires `TYPE=BepInEx`.                           |
| `GALE_SYNC_CONFIGS` | `false`                           | Set to `true` to copy the profile's `BepInEx/config` files onto the server. |
| `GALE_SYNC_URL`     | `https://gale.kesomannen.com/api` | Sync API base URL. Only change this for a self-hosted Gale sync server.     |
