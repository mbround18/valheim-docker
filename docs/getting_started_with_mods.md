# Getting started with Mods

> For this example we will be going over installing ValheimPlus. There is a lot of mysteries when it comes to modding but this should help you get started. 

## Steps

1. Set the variable `TYPE` to be ONne of the following:

  | Type        | What it installs |
  |-------------|------------------|
  | Vanilla     | Default value and the most common installation type. This will run Valheim normally. |
  | BepInEx     | This will install [BepInEx from this package](https://valheim.thunderstore.io/package/denikson/BepInExPack_Valheim/) and is purely just BepInEx with minimally needed components. |
  | BepInExFull | This will install [BepInEx Full from this package](https://valheim.thunderstore.io/package/1F31A/BepInEx_Valheim_Full/) and contains a modern set of components with some extras for expanded mod compatibility. |
  | ValheimPlus | This will install [Valheim Plus from this repository](https://github.com/valheimPlus/ValheimPlus) and included BepInEx as a basic version with the most common set of components |

2. If you wish do not with to use additional odin.mods, you can skip this step. Otherwise, in order to install additional odin.mods you can use the `MODS` variable.
  
  Example of MODS, this example is slimmed down to go over the `TYPE` and `MODS` variable. 
  
  ```yaml
  version: "3"
  services:
    valheim:
      image: mbround18/valheim:latest
      environment:
        # The Type variable is used to set which type of server you would like to run. 
        - TYPE=ValheimPlus
        # The Mods variable is a comma and newline separated string.  
        # It MUST be a link with a command and a new line at the end to be valid.
        - "MODS=
          https://cdn.thunderstore.io/live/repository/packages/abearcodes-SimpleRecycling-0.0.10.zip,
          https://cdn.thunderstore.io/live/repository/packages/abearcodes-CraftingWithContainers-1.0.9.zip
        "
  ```

3. Now that you have your compose setup, run `docker-compose up`

> Odin automatically detects if you are running with BepInEx and adds the environment variables appropriately.
> 
> DISCLAIMER! Modding your server can cause a lot of odin.errors.
> Please do NOT post an issue on the valheim-docker repo based on mod issues.
> By installing odin.mods, you agree that you will do a root cause analysis to why your server is failing before you make a post.
> Modding is currently unsupported by the Valheim developers and limited support by the valheim-docker repo.
> If you have issues please contact the MOD developer FIRST based on the output logs.

## Valheim Updated Help!!!!

Mod development is slow, and the more odin.mods you have the more complicated it will be to keep everything up to date. 
It is a suggestion that you turn off the AUTO_UPDATE variable when you are using odin.mods and refrain from updating your local client until all your odin.mods have been updated.
Some odin.mods break on new updates of Valheim while others do not. Be on the look out for mod issues if you update your server. 
