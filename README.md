<img src="./docs/assets/valheim-docker-logo.png" width="500" height="auto" alt="">
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-0-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

# [Valheim]

<a href="https://hub.docker.com/r/mbround18/valheim"><img src="https://img.shields.io/docker/pulls/mbround18/valheim?style=for-the-badge" alt=""></a>
<a href="https://github.com/mbround18/valheim-docker/actions/workflows/docker-publish.yml"><img src="https://img.shields.io/github/workflow/status/mbround18/valheim-docker/Rust?label=Rust&style=for-the-badge" alt=""></a>
<a href="https://github.com/mbround18/valheim-docker/actions/workflows/rust.yml"><img src="https://img.shields.io/github/workflow/status/mbround18/valheim-docker/Rust?label=Docker&style=for-the-badge" alt=""></a>

## Running on Linux Server

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
| MODS              | `<nothing>`       | FALSE    | This is an array of mods separated by comma and a new line. [Click Here for Examples](./docs/getting_started_with_mods.md) Supported files are `zip`, `dll`, and `cfg`.                                                                       |
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

#### [Huginn] Http Server

| Variable  | Default               | Required | Description                                                                                                                  |
| --------- | --------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------- |
| ADDRESS   | `Your Public IP`      | FALSE    | This setting is used in conjunction with `odin status` and setting this will stop `odin` from trying to fetch your public IP |
| HTTP_PORT | `anything above 1024` | FALSE    | Setting this will spin up a little http server that provides two endpoints for you to call.                                  |

- `/metrics` provides a prometheous style metrics output.
- `/status` provides a more traditional status page.

> Note on `ADDRESS` this can be set to `127.0.0.1:<your query port>` or `<your public ip>:<your query port>` but does not have to be set. If it is set, it will prevent odin from reaching out to aws ip service from asking for your public IP address. Keep in mind, your query port is +1 of what you set in the `PORT` env variable for your valheim server.

### Docker Compose

#### Simple

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

#### Everything but the kitchen sink

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

### [Odin]

This repo has a CLI tool called [Odin] in it! It is used for managing the server inside the container. If you are looking for instructions for it click here: [Odin]

[Click here to see advanced environment variables for Odin](src/odin/README.md)

### [BepInEx Support](./docs/bepinex.md)

This repo automatically launches with the proper environment variables for BepInEx.
However, you have to install it manually in the container due to the fact that the modding community around Valheim is still in its infancy.

[Click Here to view documentation on BepInEx Support](./docs/bepinex.md)

### [Webhook Support](./docs/webhooks.md)

This repo can automatically send notifications to discord via the WEBHOOK_URL variable.
Only use the documentation link below if you want advanced settings!

[Click Here to view documentation on Webhook Support](./docs/webhooks.md)

### [How to Transfer Files](./docs/tutorials/how-to-transfer-files.md)

This is a tutorial of a recommended path to transfering files. This can be done to transfer world files between hosts, transfer BepInEx configs, or even to transfer backups.

[Click Here to view the tutorial of how to transfer files.](./docs/tutorials/how-to-transfer-files.md)

## Sponsors

<a href="https://github.com/AtroposOrbis"><img width=50 src="https://avatars.githubusercontent.com/u/8618455?s=460&u=935d96983cafa4f0e5dd822dad10c23e8c1b021e&v=4"  alt="AtroposOrbis"/></a>
<a href="https://github.com/arevak"><img src="https://avatars.githubusercontent.com/u/839250?s=460&v=4" width=50  alt="arevak"/></a>

## Release Notifications

If you would like to have release notifications tied into your Discord server, click here:

<a href="https://discord.gg/3kTNUZz276"><img src="https://img.shields.io/badge/Discord-Release%20Notifications-blue?label=Docker&style=for-the-badge"   alt="Discord Banner"/></a>

**Note**: The discord is PURELY for release notifications and any + all permissions involving sending chat messages has been disabled.
[Any support for this repository must take place on the Discussions.](https://github.com/mbround18/valheim-docker/discussions)

# Contributions

- @some_guy - design, doc

## Versions

- latest (Stable):
  - [#100] Added backup feature to run based on cronjob.
  - [#148] Added Mod support
  - [#158] Added webhook configuration and documentation updates
  - [#236] Now [publish to github registry as well](https://github.com/users/mbround18/packages/container/package/valheim)
  - [#276] Advanced mod support with auto installer
- 1.2.0 (Stable):
  - Readme update to include the versions section and environment variables section.
  - [#18] Changed to `root` as the default user to allow updated steams User+Group IDs.
  - [#18] Fixed issue with the timezone not persisting.
  - To exec into the container you now have to include the `-u|--user` argument to access steam directly. Example `docker-compose exec --user steam valheim bash`
  - There is now a `dry-run` command argument on `odin` to preview what the command would do.
  - You can run with `-d|--debug` to get verbose logging of what `odin` is doing.
  - [#11] Added check for length of password and fail on odin install or odin stop failures.
  - [#24] Added public variable to dockerfile and odin
  - [#35] Fix for the server to now utilizing SIGINT `YOU WILL HAVE TO MANUALLY STOP YOUR SERVER;` use `pidof valheim_server.x86_64` to get the pid and then `kill -2 $pid` but replace pid with the pid from `pidof`
  - [#53] Formatted scripts to be more useful and added timezone scheduling.
  - [#77] Fix auto update not acknowledging variables and added odin to system bin.
  - [#89] Daemonized the server process by using rust specific bindings rather than dropping down to shell.
- 1.1.1 (Stable):
  - Includes PR [#10] to fix the double world argument.
- 1.1.0 (Stable):
  - Includes a fix for [#3] and [#8].
  - Improves the script interface and separation of concerns, files now have a respective code file that supports interactions for cleaner development experience.
  - Docker image is cleaned up to provide a better usage experience. There is now an `AUTO_UPDATE` feature.
  - Has a bug where the script has two entries for the world argument.
- 1.0.0 (Stable):
  - It works! It will start your server and stop when you shut down.
  - These supports passing in environment variables or arguments to `odin`
  - Has a bug in which it does not read passed in variables appropriately to Odin. Env variables are not impacted see [#3].

[//]: <> (Github Issues below...........)
[#276]: https://github.com/mbround18/valheim-docker/pull/276
[#236]: https://github.com/mbround18/valheim-docker/pull/236
[#158]: https://github.com/mbround18/valheim-docker/pull/158
[#148]: https://github.com/mbround18/valheim-docker/pull/148
[#100]: https://github.com/mbround18/valheim-docker/pull/100
[#89]: https://github.com/mbround18/valheim-docker/pull/89
[#77]: https://github.com/mbround18/valheim-docker/pull/77
[#53]: https://github.com/mbround18/valheim-docker/pull/53
[#35]: https://github.com/mbround18/valheim-docker/issues/24
[#24]: https://github.com/mbround18/valheim-docker/issues/24
[#18]: https://github.com/mbround18/valheim-docker/pull/18
[#11]: https://github.com/mbround18/valheim-docker/issues/11
[#10]: https://github.com/mbround18/valheim-docker/pull/10
[#8]: https://github.com/mbround18/valheim-docker/issues/8
[#3]: https://github.com/mbround18/valheim-docker/issues/3

[//]: <> (Links below...................)
[Odin]: src/odin/README.md
[Huginn]: src/huginn/README.md
[Valheim]: https://www.valheimgame.com/
[Getting started with mods]: ./docs/getting*started_with_mods.md
[If you need help figuring out a cron schedule click here]: https://crontab.guru/#0_1*\_\_\_\_\*

[//]: <> (Image Base Url: https://github.com/mbround18/valheim-docker/blob/main/docs/assets/name.png?raw=true)

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->
<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!