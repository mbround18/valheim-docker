# Odin

Odin is a CLI tool utilized for installing, starting, and stopping [Valheim] servers

## Gotchas

- Odin relies on Rust. [Please install Rust](https://www.rust-lang.org/tools/install)
- Odin also assumes that you have SteamCMD already installed. [Install instructions for SteamCMD.](https://developer.valvesoftware.com/wiki/SteamCMD)
- If you have the proper build tools installed you should be able to run Odin on any system.
- Current Supported Architecture: Unix & Linux based systems.

## Installation

> Make sure you have build essentials installed before you install this crate

```sh
cargo install --git https://github.com/mbround18/valheim-docker.git --branch main
```

## Usage

![Main Menu](./assets/main-menu.png)

#### Install Valheim

```sh
odin install
```

![Install Menu](./assets/install-menu.png)

### Start Valheim

```sh
odin start
```

![Start Menu](./assets/start-menu.png)

### Stop Valheim

```sh
odin stop
```

![Install Menu](./assets/stop-menu.png)
