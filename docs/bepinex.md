# BepInEx Support

> [As of March 2021, this repo has an environment flag to run with BepInEx. Learn more](tutorials/getting_started_with_mods.md)

## Installing BepInEx

> Due to the fact that there are so many variants of installing and running BepInEx; we will be covering only the basics.
> If you have additional questions, please visit their Discord. [BepInEx Discord](https://discord.gg/aZszQ9YB)
> If you have issues with setting up a specific plugin, please contact the mod developer!

1. Access the container as the steam user.

   ```sh
   docker compose exec --user steam valheim bash
   ```

2. Create a new folder

   ```sh
   mkdir -p ~/tmp
   ```

3. Download BepInEx

   ```sh
   wget -O /home/steam/tmp/bepinex.zip https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x64_5.4.23.2.zip
   ```

4. Extract the BepInEx zip file

   ```shell
   unzip -o /home/steam/tmp/bepinex.zip -d /home/steam/valheim
   ```

5. Cleanup files

   ```shell
   cd /home/steam/valheim && rm -rf /home/steam/tmp
   ```

6. Restart your server.

> You should see a disclaimer in your console about running with BepInEx.

## BepInEx/Modded Variables

> [Odin] auto-detects your BepInExPack_Valheim version from `BepInEx/manifest.json` and sets the correct environment for Doorstop 3.x (legacy) or 4.x+.
> You rarely need to set these yourself—treat them as advanced overrides.

Threshold: Doorstop 4.x is used starting at version `5.4.2330` and newer. Versions `5.4.2202` and earlier use legacy (Doorstop 3.x) variables.

Common (both modes)

| Variable        | Default                     | Required | Description                                                     |
| --------------- | --------------------------- | -------- | --------------------------------------------------------------- |
| LD_PRELOAD      | `libdoorstop_x64.so`        | TRUE     | Library to preload on Valheim start.                            |
| LD_LIBRARY_PATH | `./linux64:<doorstop_libs>` | TRUE     | Library search paths. `doorstop_libs` is resolved automatically |

Doorstop 3.x (<= 5.4.2202)

| Variable                      | Default                                                  | Required | Description                                                              |
| ----------------------------- | -------------------------------------------------------- | -------- | ------------------------------------------------------------------------ |
| DOORSTOP_ENABLE               | `TRUE`                                                   | TRUE     | Enables Doorstop (legacy).                                               |
| DOORSTOP_INVOKE_DLL_PATH      | `/home/steam/valheim/BepInEx/core/BepInEx.Preloader.dll` | TRUE     | BepInEx preload DLL to invoke (legacy).                                  |
| DOORSTOP_CORLIB_OVERRIDE_PATH | `/home/steam/valheim/unstripped_corlib`                  | TRUE     | Location of unstripped corlib (legacy); falls back to `BepInEx/core_lib` |

Doorstop 4.x (>= 5.4.2330)

| Variable                 | Default                                                  | Required | Description                                        |
| ------------------------ | -------------------------------------------------------- | -------- | -------------------------------------------------- |
| DOORSTOP_ENABLED         | `1`                                                      | TRUE     | Enables Doorstop (4.x+).                           |
| DOORSTOP_TARGET_ASSEMBLY | `/home/steam/valheim/BepInEx/core/BepInEx.Preloader.dll` | TRUE     | Target assembly to load (replaces INVOKE_DLL_PATH) |

Advanced (optional overrides)

| Variable      | Default                                                                       | Required | Description                                                     |
| ------------- | ----------------------------------------------------------------------------- | -------- | --------------------------------------------------------------- |
| DOORSTOP_LIB  | `libdoorstop_x64.so`                                                          | Optional | Which Doorstop library to load; Odin uses this to build preload |
| DOORSTOP_LIBS | `/home/steam/valheim/doorstop_libs` or `/home/steam/valheim/BepInEx/doorstop` | Optional | Where to look for Doorstop libs; Odin derives LD_LIBRARY_PATH   |

[odin]: ../src/odin/README.md
