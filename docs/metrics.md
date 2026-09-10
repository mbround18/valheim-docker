# Metrics

Huginn serves Prometheus metrics at `http://<host>:<HTTP_PORT>/metrics` (default port `3000`). All metrics are gauges. Server metrics come from the Steam A2S query, player metrics from Odin's player tracking (see [players](#players)), system metrics from the container's view of the host.

## Server

Labels on every server metric: `name` (server name), `version` (Steam server version tag), `map` (world name).

| Metric                         | Labels                 | Description                                                        |
| ------------------------------ | ---------------------- | ------------------------------------------------------------------ |
| `valheim_online`               | `name`,`version`,`map` | `1` when the server answers the Steam query, `0` when it does not. |
| `valheim_current_player_count` | `name`,`version`,`map` | Players connected.                                                 |
| `valheim_max_player_count`     | `name`,`version`,`map` | Player slots.                                                      |
| `valheim_bepinex_installed`    | `name`,`version`,`map` | `1` when BepInEx is installed, else `0`.                           |

## Players

One series per player currently online. Names come from the server log (`Got character ZDOID from <name>`), which Odin tracks in `player.list`; Valheim does not publish names over the Steam query. Series disappear when the player leaves.

| Metric                                    | Labels   | Description                                                   |
| ----------------------------------------- | -------- | ------------------------------------------------------------- |
| `valheim_player_online`                   | `player` | Always `1` while the character is online.                     |
| `valheim_player_joined_timestamp_seconds` | `player` | Unix time the player joined. Kept across deaths and respawns. |

Time in game: `time() - valheim_player_joined_timestamp_seconds`.

## System

| Metric                             | Labels   | Description                                    |
| ---------------------------------- | -------- | ---------------------------------------------- |
| `valheim_sys_memory_total_bytes`   |          | Total memory.                                  |
| `valheim_sys_memory_used_bytes`    |          | Memory in use.                                 |
| `valheim_sys_swap_total_bytes`     |          | Total swap.                                    |
| `valheim_sys_swap_used_bytes`      |          | Swap in use.                                   |
| `valheim_sys_disk_total_bytes`     |          | Total size of all mounted disks.               |
| `valheim_sys_disk_available_bytes` |          | Free space across all mounted disks.           |
| `valheim_sys_cpu_logical_count`    |          | Logical CPUs.                                  |
| `valheim_sys_load_average`         | `window` | Load average; `window` is `1m`, `5m` or `15m`. |

## Example

```
valheim_online{name="My Server", version="g=1.0.7,n=39", map="Dedicated"} 1
valheim_current_player_count{name="My Server", version="g=1.0.7,n=39", map="Dedicated"} 2
valheim_max_player_count{name="My Server", version="g=1.0.7,n=39", map="Dedicated"} 10
valheim_bepinex_installed{name="My Server", version="g=1.0.7,n=39", map="Dedicated"} 0
valheim_sys_memory_total_bytes 8317225140
valheim_sys_memory_used_bytes 6340026040
valheim_sys_swap_total_bytes 0
valheim_sys_swap_used_bytes 0
valheim_sys_disk_total_bytes 833209548800
valheim_sys_disk_available_bytes 450985697280
valheim_sys_cpu_logical_count 4
valheim_sys_load_average {window="1m"} 0.98
valheim_sys_load_average {window="5m"} 0.94
valheim_sys_load_average {window="15m"} 1.01
valheim_player_online{player="Viking"} 1
valheim_player_joined_timestamp_seconds{player="Viking"} 1789020710
```

Scrape it like any other target (Prometheus `static_configs`, or a Kubernetes `ServiceMonitor` on the Huginn port). A Grafana dashboard walkthrough is in [discussion #330](https://github.com/mbround18/valheim-docker/discussions/330).
