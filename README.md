<img src="./docs/assets/valheim-docker-logo.png" width="500" height="auto" alt="">

# [Valheim]

<a href="https://hub.docker.com/r/mbround18/valheim"><img src="https://img.shields.io/docker/pulls/mbround18/valheim?style=for-the-badge" alt=""></a>
<a href="https://github.com/mbround18/valheim-docker/actions/workflows/docker-publish.yml"><img src="https://img.shields.io/github/workflow/status/mbround18/valheim-docker/Rust?label=Rust&style=for-the-badge" alt=""></a>
<a href="https://github.com/mbround18/valheim-docker/actions/workflows/rust.yml"><img src="https://img.shields.io/github/workflow/status/mbround18/valheim-docker/Rust?label=Docker&style=for-the-badge" alt=""></a>
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-8-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

## Table of Contents

* [Running on a bare-metal Linux Server](#running-on-a-bare-metal-linux-server)
  * [From Release](#from-release)
  * [From Source](#from-source)
* [Running with Docker](#running-with-docker)
  * [Download Locations](#download-locations)
    * [DockerHub](#dockerhub)
    * [GitHub Container Registry](#github-container-registry)
  * [Environment Variables](#environment-variables)
    * [Container Env Variables](#container-env-variables)
    * [Auto Update](#auto-update)
    * [Auto Backup](#auto-backup)
* [Docker Compose](#docker-compose)
  * [Simple](#simple)
  * [Everything but the kitchen sink](#everything-but-the-kitchen-sink)
* [Bundled Tools](#bundled-tools)
  * [Odin](#odin)
  * [Huginn Http Server](#huginn-http-server)
* [Feature Information](#feature-information)
  * [BepInEx Support](#bepinex-support)
  * [Webhook Support](#webhook-support)
* [Guides](#guides)
  > Did you write a guide? or perhaps an article? Add a PR to have it added here in the readme <3
  * [How to Transfer Files](#how-to-transfer-files)
  * [External: Hosting Valheim on Rocket Pi X](https://ikarus.sg/valheim-server-rock-pi-x/)
  * [External: Valheim on AWS](https://aws.amazon.com/getting-started/hands-on/valheim-on-aws/)
  * [External: How to host a dedicated Valheim server on Amazon Lightsail](https://updateloop.dev/dedicated-valheim-lightsail/)
  * [External: Experience With Valheim Game Hosting With Docker](https://norton-setup.support/games/experience-with-valheim-game-hosting-with-docker/)
* [Additional Information](#additional-information)
  * [Discord Release Notifications](#discord-release-notifications)
  * [Versions](#versions)
* [❤️ Sponsors ❤️](#sponsors)
* [✨ Contributors ✨](#contributors-)

## Running on a bare-metal Linux Server

### From Release

1. Navigate to `https://github.com/mbround18/valheim-docker/releases/latest`
2. Download the `bundle.zip` to your server
3. Extract the `bundle.zip`
4. Make the files executable `chmod +x {odin,huginn}`
5. Optional: Add the files to your path. 
6. Navigate to the folder where you want your server installed. 
7. Run `odin configure --password "Your Super Strong Password"` (you can also supply `--name "Server Name"`, `--port "Server Port"`, or other arguments available.)
8. Finally, run `odin start`. 

**More in-depth How-to Article:** https://dev.to/mbround18/running-valheim-on-an-linux-server-4kh1

### From Source

This repo bundles its tools in a way that you can run them without having to install docker!
If you purely want to run this on a Linux based system, without docker, take a look at the links below <3

- [Installing & Using Odin](./src/odin/README.md)
  The tool [Odin] runs the show and does almost all the heavy lifting in this repo. It starts, stops, and manages your Valheim server instance.
- [Installing & Using Huginn](./src/huginn/README.md)
  Looking for a way to view the status of your server? Look no further than [Huginn]!
  The [Huginn] project is a http server built on the same source as [Odin] and uses these capabilities to expose a few http endpoints.

> Using the binaries to run on an Ubuntu Server, you will have to be more involved and configure a few things manually.
> If you want a managed, easy one-two punch to manage your server. Then look at the Docker section <3

## Running with Docker

> [If you are looking for a guide on how to get started click here](https://github.com/mbround18/valheim-docker/discussions/28)
>
> Mod Support! It is supported to launch the server with BepInEx but!!!!! as a disclaimer! You take responsibility for debugging why your server won't start.
> Modding is not supported by the Valheim developers officially yet; Which means you WILL run into errors. This repo has been tested with running ValheimPlus as a test mod and does not have any issues.
> See [Getting started with mods]

### Download Locations

#### DockerHub

<a href="https://hub.docker.com/r/mbround18/valheim"><img alt="DockerHub Valheim" src="https://img.shields.io/badge/DockerHub-Valheim-blue?style=for-the-badge"></a>
<a href="https://hub.docker.com/r/mbround18/valheim-odin"><img alt="DockerHub Odin" src="https://img.shields.io/badge/DockerHub-Odin-blue?style=for-the-badge"></a>

#### GitHub Container Registry

<a href="https://github.com/users/mbround18/packages/container/package/valheim"><img alt="GHCR Valheim" src="https://img.shields.io/badge/GHCR-Valheim-blue?style=for-the-badge"></a>
<a href="https://github.com/users/mbround18/packages/container/package/valheim-odin"><img alt="GHCR Odin" src="https://img.shields.io/badge/GHCR-Odin-blue?style=for-the-badge"></a>

### Environment Variables

> See further on down for advanced environment variables.

| Variable          | Default           | Required | Description                                                                                                                                                                                                                                   |
| ----------------- | ----------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PORT              | `2456`            | TRUE     | Sets the port your server will listen on. Take note it will also listen on +2 (ex: 2456, 2457, 2458)                                                                                                                                          |
| NAME              | `Valheim Docker`  | TRUE     | The name of your server! Make it fun and unique!                                                                                                                                                                                              |
| WORLD             | `Dedicated`       | TRUE     | This is used to generate the name of your world.                                                                                                                                                                                              |
| PUBLIC            | `1`               | FALSE    | Sets whether or not your server is public on the server list.                                                                                                                                                                                 |
| PASSWORD          | `<please set me>` | TRUE     | Set this to something unique!                                                                                                                                                                                                                 |
| TYPE              | `Vanilla`         | FALSE    | This can be set to `ValheimPlus`, `BepInEx`, `BepInExFull` or `Vanilla`                                                                                                                                                                       |
| MODS              | `<nothing>`       | FALSE    | This is an array of mods separated by comma and a new line. [Click Here for Examples](./docs/tutorials/getting_started_with_mods.md) Supported files are `zip`, `dll`, and `cfg`.                                                                       |
| WEBHOOK_URL       | `<nothing>`       | FALSE    | Supply this to get information regarding your server's status in a webhook or Discord notification! [Click here to learn how to get a webhook url for Discord](https://help.dashe.io/en/articles/2521940-how-to-create-a-discord-webhook-url) |
| UPDATE_ON_STARTUP | `1`               | FALSE    | Tries to update the server the container is started.                                                                                                                                                                                          |

#### Container Env Variables

| Variable | Default               | Required | Description                                                                                                                                                                                           |
| -------- | --------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TZ       | `America/Los_Angeles` | FALSE    | Sets what timezone your container is running on. This is used for timestamps and cron jobs. [Click Here for which timezones are valid.](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) |
| PUID     | `1000`                | FALSE    | Sets the User Id of the steam user.                                                                                                                                                                   |
| PGID     | `1000`                | FALSE    | Sets the Group Id of the steam user.                                                                                                                                                                  |

#### Auto Update

| Variable                       | Default     | Required | Description                                                                                                                                                                                                                                                                     |
| ------------------------------ | ----------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AUTO_UPDATE                    | `0`         | FALSE    | Set to `1` if you want your container to auto update! This means at the times indicated by `AUTO_UPDATE_SCHEDULE` it will check for server updates. If there is an update then the server will be shut down, updated, and brought back online if the server was running before. |
| AUTO_UPDATE_SCHEDULE           | `0 1 * * *` | FALSE    | This works in conjunction with `AUTO_UPDATE` and sets the schedule to which it will run an auto update. [If you need help figuring out a cron schedule click here]                                                                                                              |
| AUTO_UPDATE_PAUSE_WITH_PLAYERS | `0`         | FALSE    | Does not process an update for the server if there are players online.                                                                                                                                                                                                          |

Auto update job, queries steam and compares it against your internal steam files for differential in version numbers.

#### Auto Backup

| Variable                          | Default        | Required | Description                                                                                                                                                 |
| --------------------------------- | -------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AUTO_BACKUP                       | `0`            | FALSE    | Set to `1` to enable auto backups. Backups are stored under `/home/steam/backups` which means you will have to add a volume mount for this directory.       |
| AUTO_BACKUP_SCHEDULE              | `*/15 * * * *` | FALSE    | Change to set how frequently you would like the server to backup. [If you need help figuring out a cron schedule click here].                               |
| AUTO_BACKUP_REMOVE_OLD            | `1`            | FALSE    | Set to `0` to keep all backups or manually manage them.                                                                                                     |
| AUTO_BACKUP_DAYS_TO_LIVE          | `3`            | FALSE    | This is the number of days you would like to keep backups for. While backups are compressed and generally small it is best to change this number as needed. |
| AUTO_BACKUP_ON_UPDATE             | `0`            | FALSE    | Create a backup on right before updating and starting your server.                                                                                          |
| AUTO_BACKUP_ON_SHUTDOWN           | `0`            | FALSE    | Create a backup on shutdown.                                                                                                                                |
| AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS | `0`            | FALSE    | Will skip creating a backup if there are no players. `PUBLIC` must be set to `1` for this to work!                                                          |

Auto backup job produces an output of a `*.tar.gz` file which should average around 30mb for a world that has an average of 4 players consistently building on. You should be aware that if you place the server folder in your saves folder your backups could become astronomical in size. This is a common problem that others have observed, to avoid this please follow the guide for how volume mounts should be made in the `docker-compose.yml`.

## Docker Compose

### Simple

> This is a basic example of a docker compose, you can apply any of the variables above to the `environment` section below but be sure to follow each variables' description notes!

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:latest
    ports:
      - 2456:2456/udp
      - 2457:2457/udp
      - 2458:2458/udp
    environment:
      PORT: 2456
      NAME: "Created With Valheim Docker"
      WORLD: "Dedicated"
      PASSWORD: "Banana Phone"
      TZ: "America/Chicago"
      PUBLIC: 1
    volumes:
      - ./valheim/saves:/home/steam/.config/unity3d/IronGate/Valheim
      - ./valheim/server:/home/steam/valheim
```

### Everything but the kitchen sink

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:latest
    ports:
      - 2456:2456/udp
      - 2457:2457/udp
      - 2458:2458/udp
    environment:
      PORT: 2456
      NAME: "Created With Valheim Docker"
      WORLD: "Dedicated"
      PASSWORD: "Strong! Password @ Here"
      TZ: "America/Chicago"
      PUBLIC: 1
      AUTO_UPDATE: 1
      AUTO_UPDATE_SCHEDULE: "0 1 * * *"
      AUTO_BACKUP: 1
      AUTO_BACKUP_SCHEDULE: "*/15 * * * *"
      AUTO_BACKUP_REMOVE_OLD: 1
      AUTO_BACKUP_DAYS_TO_LIVE: 3
      AUTO_BACKUP_ON_UPDATE: 1
      AUTO_BACKUP_ON_SHUTDOWN: 1
      WEBHOOK_URL: "https://discord.com/api/webhooks/IM_A_SNOWFLAKE/AND_I_AM_A_SECRET"
      UPDATE_ON_STARTUP: 0
    volumes:
      - ./valheim/saves:/home/steam/.config/unity3d/IronGate/Valheim
      - ./valheim/server:/home/steam/valheim
      - ./valheim/backups:/home/steam/backups
```
## Bundled Tools

### [Odin]

This repo has a CLI tool called [Odin] in it! It is used for managing the server inside the container. If you are looking for instructions for it click here: [Odin]

[Click here to see advanced environment variables for Odin](src/odin/README.md)


### [Huginn] Http Server

| Variable  | Default               | Required | Description                                                                                                                  |
| --------- | --------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------- |
| ADDRESS   | `Your Public IP`      | FALSE    | This setting is used in conjunction with `odin status` and setting this will stop `odin` from trying to fetch your public IP |
| HTTP_PORT | `anything above 1024` | FALSE    | Setting this will spin up a little http server that provides two endpoints for you to call.                                  |

- `/metrics` provides a prometheous style metrics output.
- `/status` provides a more traditional status page.

> Note on `ADDRESS` this can be set to `127.0.0.1:<your query port>` or `<your public ip>:<your query port>` but does not have to be set. If it is set, it will prevent odin from reaching out to aws ip service from asking for your public IP address. Keep in mind, your query port is +1 of what you set in the `PORT` env variable for your valheim server.

## Feature Information

### [BepInEx Support](./docs/bepinex.md)

As of [March 2021](./docs/bepinex.md) the TYPE variable can be used to automatically install BepInEx. For details see [Getting started with mods].

### [Webhook Support](./docs/webhooks.md)

This repo can automatically send notifications to discord via the WEBHOOK_URL variable.
Only use the documentation link below if you want advanced settings!

[Click Here to view documentation on Webhook Support](./docs/webhooks.md)

## Guides

### [How to Transfer Files](./docs/tutorials/how-to-transfer-files.md)

This is a tutorial of a recommended path to transfering files. This can be done to transfer world files between hosts, transfer BepInEx configs, or even to transfer backups.

[Click Here to view the tutorial of how to transfer files.](./docs/tutorials/how-to-transfer-files.md)


## Additional Information

### Discord Release Notifications

If you would like to have release notifications tied into your Discord server, click here:

<a href="https://discord.gg/3kTNUZz276"><img src="https://img.shields.io/badge/Discord-Release%20Notifications-blue?label=Docker&style=for-the-badge"   alt="Discord Banner"/></a>

**Note**: The discord is PURELY for release notifications and any + all permissions involving sending chat messages has been disabled.
[Any support for this repository must take place on the Discussions.](https://github.com/mbround18/valheim-docker/discussions)

### Versions

- latest (Stable): Mod support! and cleaned up the code base.
- 1.4.x (Stable): Webhook for discord upgrade.
- 1.3.x (Stable): Health of codebase improvements.
- 1.2.0 (Stable): Added additional stop features and sig for stopping. 
- 1.1.1 (Stable): Patch to fix arguments
- 1.1.0 (Unstable): Cleaned up image and made it faster
- 1.0.0 (Stable): It works! 

[//]: <> (Links below...................)
[Odin]: src/odin/README.md
[Huginn]: src/huginn/README.md
[Valheim]: https://www.valheimgame.com/
[Getting started with mods]: ./docs/tutorials/getting_started_with_mods.md
[If you need help figuring out a cron schedule click here]: https://crontab.guru/#0_1*\_\_\_\_\*

[//]: <> (Image Base Url: https://github.com/mbround18/valheim-docker/blob/main/docs/assets/name.png?raw=true)

## Sponsors

<a href="https://github.com/arevak"><img src="https://avatars.githubusercontent.com/u/839250?s=460&v=4" width=50  alt="arevak"/></a>

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="http://arneman.me/"><img src="https://avatars.githubusercontent.com/u/3298808?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Mark</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=bearlikelion" title="Documentation">📖</a></td>
    <td align="center"><a href="https://m.bruno.fyi/"><img src="https://avatars.githubusercontent.com/u/12646562?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Michael</b></sub></a><br /><a href="#infra-mbround18" title="Infrastructure (Hosting, Build-Tools, etc)">🚇</a> <a href="https://github.com/mbround18/valheim-docker/commits?author=mbround18" title="Code">💻</a> <a href="https://github.com/mbround18/valheim-docker/commits?author=mbround18" title="Documentation">📖</a></td>
    <td align="center"><a href="https://github.com/apps/imgbot"><img src="https://avatars.githubusercontent.com/in/4706?v=4?s=100" width="100px;" alt=""/><br /><sub><b>imgbot[bot]</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=imgbot[bot]" title="Documentation">📖</a></td>
    <td align="center"><a href="https://github.com/AGhost-7"><img src="https://avatars.githubusercontent.com/u/6957411?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Jonathan Boudreau</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=AGhost-7" title="Code">💻</a></td>
    <td align="center"><a href="https://github.com/Kellei2983"><img src="https://avatars.githubusercontent.com/u/32897629?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Lukáš Hruška</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=Kellei2983" title="Documentation">📖</a></td>
    <td align="center"><a href="http://vallee-design.de/"><img src="https://avatars.githubusercontent.com/u/6720458?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Julian Vallée</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=JulianVallee" title="Code">💻</a></td>
    <td align="center"><a href="https://github.com/Finomnis"><img src="https://avatars.githubusercontent.com/u/3129043?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Finomnis</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=Finomnis" title="Code">💻</a></td>
  </tr>
  <tr>
    <td align="center"><a href="https://tech.jrlbyrne.com/"><img src="https://avatars.githubusercontent.com/u/14056930?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Justin Byrne</b></sub></a><br /><a href="https://github.com/mbround18/valheim-docker/commits?author=JustinByrne" title="Documentation">📖</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
