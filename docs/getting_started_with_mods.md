# Getting started with Mods

> For this example we will be going over installing ValheimPlus. There is a lot of mysteries when it comes to modding but this should help you get started. 

## Steps

1. Download the mod file, for our example we want the `UnixServer.zip` from `https://github.com/nxPublic/ValheimPlus/releases`
2. Place the file in your server volume mount. `cp UnixServer.zip /home/youruser/valheim/server`
3. Unzip the archive `unzip UnixServer.zip -d .` hit A to replace all as needed.
4. Restart your server.

> Odin automatically detects if you are running with BepInEx and adds the environment variables appropriately.
> 
> DISCLAIMER! Modding your server can cause a lot of errors.
> Please do NOT post an issue on the valheim-docker repo based on mod issues.
> By installing mods, you agree that you will do a root cause analysis to why your server is failing before you make a post.
> Modding is currently unsupported by the Valheim developers and limited support by the valheim-docker repo.
> If you have issues please contact the MOD developer FIRST based on the output logs.

## Valheim Updated Help!!!!

Mod development is slow, and the more mods you have the more complicated it will be to keep everything up to date. 
It is a suggestion that you turn off the AUTO_UPDATE variable when you are using mods and refrain from updating your local client until all your mods have been updated. 
