# Installing ValheimPlus

ValheimPlus is a comprehensive mod that enhances Valheim with numerous quality-of-life improvements and features. This guide walks you through installing it on your Valheim Docker server.

## Quick Start

Set `TYPE` to `ValheimPlus`:

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    user: "1000:1000"
    environment:
      - TYPE=ValheimPlus
```

That is it. The server installs BepInEx, downloads the latest `ValheimPlus.dll` into
`BepInEx/plugins/`, and downloads the matching `valheim_plus.cfg` into `BepInEx/config/` on
startup.

## Choosing a Version

By default `TYPE=ValheimPlus` follows the newest release of
[Grantapher's ValheimPlus](https://github.com/Grantapher/ValheimPlus), the maintained fork,
and picks up new releases when the container restarts. That is the same policy the image
already uses for BepInEx itself.

To stay on a known version instead, pin it:

```yaml
environment:
  - TYPE=ValheimPlus
  - VALHEIM_PLUS_RELEASE=0.10.1.2
```

| Variable                    | Default                    | What it does                                                                |
| --------------------------- | -------------------------- | --------------------------------------------------------------------------- |
| `VALHEIM_PLUS_RELEASE`      | `latest`                   | The release tag to install, or `latest`.                                    |
| `VALHEIM_PLUS_REPOSITORY`   | `Grantapher/ValheimPlus`   | The GitHub repository to pull `ValheimPlus.dll` from, for other forks.      |
| `VALHEIM_PLUS_DOWNLOAD_URL` | derived from the two above | A full URL to a `ValheimPlus.dll`, used as is. Overrides both of the above. |

## How It Works

`TYPE=ValheimPlus` is `TYPE=BepInEx` plus one known plugin:

1. **BepInEx is installed** as the mod loader framework, exactly as for `TYPE=BepInEx`
2. **The ValheimPlus download URL is added to `MODS`**, so it goes through the same install
   path as any other mod: it is tracked, updated, and removed again if you change `TYPE`
3. **ValheimPlus.dll is downloaded** and placed in `BepInEx/plugins/`
4. **valheim_plus.cfg** is downloaded from the same release and placed in `BepInEx/config/`,
   unless you already have one there, which is never overwritten
5. The server starts with ValheimPlus loaded

If you list ValheimPlus in `MODS` yourself, your entry is used and nothing is added.

## Combining with Other Mods

`MODS` works as usual; ValheimPlus is appended to whatever you set:

```yaml
environment:
  - TYPE=ValheimPlus
  - |
    MODS=https://cdn.thunderstore.io/live/repository/packages/OdinPlus-OdinHorse-1.4.12.zip
    https://cdn.thunderstore.io/live/repository/packages/ValheimModding-Jotunn-2.26.0.zip
```

## Installing It by Hand

`TYPE=BepInEx` with an explicit DLL URL in `MODS` still works, and is the way to run a
build that is not a GitHub release asset:

```yaml
environment:
  - TYPE=BepInEx
  - MODS=https://github.com/Grantapher/ValheimPlus/releases/download/0.10.1.2/ValheimPlus.dll
```

## Configuration

Once ValheimPlus is installed, you can configure it by editing the configuration file:

```
{game-directory}/BepInEx/config/valheim_plus.cfg
```

This file controls all ValheimPlus features. Refer to the [ValheimPlus documentation](https://github.com/Grantapher/ValheimPlus/wiki) for available settings and what each option does.

## Combining with Other Mods

You can install ValheimPlus alongside other mods by specifying multiple URLs in the `MODS` environment variable:

```yaml
- |
  MODS=https://github.com/Grantapher/ValheimPlus/releases/download/0.9.16.2/ValheimPlus.dll
  https://cdn.thunderstore.io/live/repository/packages/OdinPlus-OdinHorse-1.4.12.zip
  https://cdn.thunderstore.io/live/repository/packages/ValheimModding-Jotunn-2.26.0.zip
```

## Troubleshooting

### Server Won't Start

- Check the server logs for errors
- Verify the ValheimPlus.dll download URL is correct and accessible
- Ensure `TYPE` is set to `ValheimPlus` (or `BepInEx` if you are listing the DLL yourself)

### Mods Not Loading

- Verify `BepInEx/plugins/` contains `ValheimPlus.dll`
- Check `BepInEx/config/valheim_plus.cfg` exists
- Review server logs for plugin load errors

### Configuration Not Applied

- The `valheim_plus.cfg` file is automatically created on first run
- If it's missing, restart the container to regenerate it
- Edit the file while the server is running; changes take effect on server restart

## Resources

- [ValheimPlus GitHub](https://github.com/Grantapher/ValheimPlus)
- [Releases Page](https://github.com/Grantapher/ValheimPlus/releases)
- [Configuration Wiki](https://github.com/Grantapher/ValheimPlus/wiki)
- [Issue Reports](https://github.com/Grantapher/ValheimPlus/issues)

## Support

If you encounter issues:

1. **Check ValheimPlus logs** in the server output
2. **Verify the mod version** matches your Valheim game version
3. **Report issues** to the [ValheimPlus project](https://github.com/Grantapher/ValheimPlus/issues), not to valheim-docker
4. **Check compatibility** with other installed mods
