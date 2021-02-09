# [Valheim]

![Rust Build](https://github.com/mbround18/valheim-docker/workflows/Rust/badge.svg)
![Docker Build](https://github.com/mbround18/valheim-docker/workflows/Docker/badge.svg)


## Docker

### Environment Variables

| Variable    | Default                | Required | Description |
|-------------|------------------------|----------|-------------|
| PUID        | `1000`                 | FALSE    | Sets the User Id of the steam user. |
| PGID        | `1000`                 | FALSE    | Sets the Group Id of the steam user. |
| NAME        | `Valheim Docker`       | TRUE     | The name of your server! Make it fun and unique! |
| WORLD       | `Dedicated`            | TRUE     | This is used to generate the name of your world. |
| PORT        | `2456`                 | TRUE     | Sets the port your server will listen on. Take not it will also listen on +2 (ex: 2456, 2457, 2458) |
| PASSWORD    | `12345`                | TRUE     | Set this to something unique! |
| TZ          | `America/Los_Angeles`  | FALSE    | Sets what timezone your container is running on. This is used for timestamps and cron jobs. [Click Here for which timezones are valid.](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) |
| AUTO_UPDATE | `0`                    | FALSE    | Set to `1` if you want your container to auto update! This means at 1 am it will update, stop, and then restart your server. |


### Docker Compose

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:latest
    ports:
      - 2456:2456/udp
      - 2457:2457/udp
      - 2458:2458/udp
    env:
      NAME: "Valheim Docker"
      WORLD: "Dedicated"
      PORT: "2456"
      PASSWORD: "something-secret"
      TZ: "America/Los_Angeles"
      AUTO_UPDATE: "0"
    volumes:
    - ./valheim/saves:/home/steam/.config/unity3d/IronGate/Valheim
    - ./valheim/server:/home/steam/valheim
```

## Odin

Odin is a CLI tool utilized for installing, starting, and stopping [Valheim] servers

### Gotchas

- Odin relies on Rust. [Please install Rust](https://www.rust-lang.org/tools/install)
- Odin also assumes that you have SteamCMD already installed. [Install instructions for SteamCMD.](https://developer.valvesoftware.com/wiki/SteamCMD)
- If you have the proper build tools installed you should be able to run Odin on any system.
- Current Supported Architecture: Unix & Linux based systems. Windows coming soon.

### Installation

> Make sure you have build essentials installed before you install this crate

```sh
cargo install --git https://github.com/mbround18/valheim-docker.git --branch main
```

### Usage

![Main Menu](./docs/assets/main-menu.png)

#### Install Valheim

```sh
odin install
```

![Install Menu](./docs/assets/install-menu.png)

#### Start Valheim

```sh
odin start
```

![Start Menu](./docs/assets/start-menu.png)

#### Stop Valheim

```sh
odin stop
```

![Install Menu](./docs/assets/stop-menu.png)

## Versions: 

- latest (Stable):
  - Readme update to include the versions section and environment variables section.
  - [#18] Changed to `root` as the default user to allow updated steams User+Group IDs.
  - [#18] Fixed issue with the timezone not persisting.
  - To exec into the container you now have to include the `-u|--user` argument to access steam directly. Example `docker-compose exec --user steam valheim bash`
  - There is now a `dry-run` command argument on `odin` to preview what the command would do. 
  - You can run with `-d|--debug` to get verbose logging of what `odin` is doing. 
- 1.1.1 (Stable): 
  - Includes PR [#10] to fix the double world argument. 
- 1.1.0 (Stable): 
  - Includes a fix for [#3] and [#8].
  - Improves the script interface and separation of concerns, files now have a respective code file that supports interactions for cleaner development experience.
  - Docker image is cleaned up to provide a better usage experience. There is now an `AUTO_UPDATE` feature.
  - Has a bug where the script has two entries for the world argument.
- 1.0.0 (Stable):
  - It works! It will start your server and stop when you shut down. 
  - This supports passing in environment variables or arguments to `odin`
  - Has a bug in which it does not read passed in variables appropriately to Odin. Env variables are not impacted see [#3]. 

[//]: <> (Github Issues below...........)
[#18]: https://github.com/mbround18/valheim-docker/pull/18
[#10]: https://github.com/mbround18/valheim-docker/pull/10
[#8]: https://github.com/mbround18/valheim-docker/issues/8
[#3]: https://github.com/mbround18/valheim-docker/issues/3 


[//]: <> (Links below...................)
[Valheim]: https://www.valheimgame.com/

[//]: <> (Image Base Url: https://github.com/mbround18/valheim-docker/blob/main/docs/assets/name.png?raw=true)
