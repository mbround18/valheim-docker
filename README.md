# [Valheim]

![Rust Build](https://github.com/mbround18/valheim-docker/workflows/Rust/badge.svg)
![Docker Build](https://github.com/mbround18/valheim-docker/workflows/Docker/badge.svg)


## Docker

### Docker Compose

```yaml
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
    volumes:
    - ./valheim/saves:/home/steam/.config/unity3d/IronGate/Valheim
    - ./valheim/server:/home/steam/valheim
```

## Odin

Odin is a CLI tool utilized for installing, starting, and stopping [Valheim] servers

### Gotchyas

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

![![Main Menu](https://github.com/mbround18/valheim-docker/blob/main/docs/assets/main-menu.png?raw=true)](./docs/assets/main-menu.png)

#### Install Valheim

```sh
odin install
```

#### Start Valheim

```sh
odin start
```

![![start menu](https://github.com/mbround18/valheim-docker/blob/main/docs/assets/start-menu.png?raw=true)](./docs/assets/start-menu.png)
#### Stop Valheim

```sh
odin stop
```


[Valheim]: https://www.valheimgame.com/
