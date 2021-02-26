# BepInEx Support

## Installing BepInEx

> Due to the fact that there are so many variants of installing and running BepInEx; we will be covering only the basics.
> If you have additional questions please visit their discord. [BepInEx Discord](https://discord.gg/aZszQ9YB)
> If you have issues with setting up a specific plugin, please contact the mod developer!

1. Access the container as the steam user.

  ```shell
  docker-compose exec --user steam valheim bash
  ```

2. Create a new folder

  ```shell
  mkdir -p ~/tmp
  ```

3. Download BepInEx

  ```shell
  wget -O /home/steam/tmp/bepinex.zip https://github.com/valheimPlus/ValheimPlus/releases/download/0.9/UnixServer.zip
  ```

4. Extract the BepInEx zip file

  > Overwrite files with `A`

  ```shell
  unzip /home/steam/tmp/bepinex.zip -d /home/steam/valheim
  
  ```

5. Cleanup files

  ```shell
  cd /home/steam/valheim && rm -rf /home/steam/tmp
  ```

6. Restart your server.

> You should see a huge disclaimer in your console about running with bepinex. 

## BepInEx/Modded Variables

> These are set automatically by [Odin] for a basic BepInEx installation;
> you DO NOT need to set these and only mess with them if you Know what you are doing.

| Variable                      | Default                                                  | Required | Description |
|-------------------------------|----------------------------------------------------------|----------|-------------|
| LD_PRELOAD                    | `libdoorstop_x64.so`                                     | TRUE     | Sets which library to preload on Valheim start. |
| LD_LIBRARY_PATH               | `./linux64:/home/steam/valheim/doorstop_libs`            | TRUE     | Sets which library paths it should look in for preload libs. | 
| DOORSTOP_ENABLE               | `TRUE`                                                   | TRUE     | Enables Doorstop or not. |
| DOORSTOP_LIB                  | `libdoorstop_x64.so`                                     | TRUE     | Which doorstop lib to load | 
| DOORSTOP_LIBS                 | `/home/steam/valheim/doorstop_libs`                      | TRUE     | Where to look for doorstop libs. | 
| DOORSTOP_INVOKE_DLL_PATH      | `/home/steam/valheim/BepInEx/core/BepInEx.Preloader.dll` | TRUE     | BepInEx preload dll to load. |
| DOORSTOP_CORLIB_OVERRIDE_PATH | `/home/steam/valheim/unstripped_corlib`                  | TRUE     | Sets where the decompiled libraries containing base mono files are located at |              
| DYLD_LIBRARY_PATH             | `"/home/steam/valheim/doorstop_libs"`                    | TRUE     | Sets the library paths. NOTE: This variable is weird and MUST have quotes around it! |
| DYLD_INSERT_LIBRARIES         | `/home/steam/valheim/doorstop_libs/libdoorstop_x64.so`   | TRUE     | Sets which library to load. |


[Odin]: ./odin.md
