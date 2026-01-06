# Installing ValheimPlus

ValheimPlus is a comprehensive mod that enhances Valheim with numerous quality-of-life improvements and features. This guide walks you through installing it on your Valheim Docker server.

## Quick Start

To install ValheimPlus, set your Docker environment variables like this:

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:latest
    environment:
      - TYPE=BepInEx
      - MODS=https://github.com/Grantapher/ValheimPlus/releases/download/0.9.16.2/ValheimPlus.dll
```

That's it! The server will automatically download the DLL, install it into BepInEx plugins, and download the ValheimPlus configuration file on startup.

## Finding the Latest Version

ValheimPlus releases are published on GitHub. To find the latest version:

1. Visit: [ValheimPlus Releases](https://github.com/Grantapher/ValheimPlus/releases)
2. Find the latest release (usually at the top of the page)
3. Look for the **Assets** section and find `ValheimPlus.dll`
4. Copy the download link URL
5. Replace the version number in the `MODS` environment variable

### Example: Updating to a Newer Version

If you see a release like `v0.9.17.0` on GitHub with a `ValheimPlus.dll` asset, update your config to:

```yaml
- MODS=https://github.com/Grantapher/ValheimPlus/releases/download/0.9.17.0/ValheimPlus.dll
```

## How It Works

When you set `TYPE=BepInEx` and include a ValheimPlus DLL URL in the `MODS` variable:

1. **BepInEx is installed** as the mod loader framework
2. **ValheimPlus.dll is downloaded** and placed in `BepInEx/plugins/`
3. **valheim_plus.cfg** is automatically downloaded from the same GitHub release and placed in `BepInEx/config/`
4. The server starts with ValheimPlus loaded and ready

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
- Ensure `TYPE=BepInEx` is set

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
