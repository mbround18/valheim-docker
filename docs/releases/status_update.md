# ❤️Status Update ❤️

## 🆕 odin status

- Can be used inside the Valheim docker container.
- Can be used to query other servers with the `--address "IP_ADDRESS"` argument.
- Will try to check for your public facing IP, to avoid this set the `ADDRESS` environment variable to be the query address. Ex: `127.0.0.1:2457`
- Has the ability to output with JSON, see below.

### Json flag

Without `--json`

```shell
[ODIN][INFO]  - Name: Creative Update
[ODIN][INFO]  - Players: 0/10
[ODIN][INFO]  - Map: Creative Update
[ODIN][INFO]  - BepInEx Enabled: true
[ODIN][INFO]  - BepInEx Mods: Example.dll
```

With `--json`

```json
{
  "name": "Fancy Name",
  "version": "0.147.3@0.9.5.5",
  "players": 0,
  "max_players": 10,
  "map": "Fancy Name",
  "online": true,
  "bepinex": {
    "enabled": true,
    "mods": []
  },
  "jobs": [
    {
      "name": "AUTO_UPDATE",
      "enabled": false,
      "schedule": "*/5 * * * *"
    },
    {
      "name": "AUTO_BACKUP",
      "enabled": true,
      "schedule": "*/5 * * * *"
    },
    {
      "name": "SCHEDULED_RESTART",
      "enabled": false,
      "schedule": "0 2 * * *"
    }
  ],
  "scheduler_state": {
    "updated_at": "2026-02-16T05:00:00-08:00",
    "jobs": []
  }
}
```

## 🆕 HTTP server for serving status

Setting the `HTTP_PORT` variable to any number will spin up a small http server that can pull server status.

You can access it via `http://127.0.0.1:HTTP_PORT/status`.
You are responsible for putting your status endpoint behind SSL or authentication if you desire.

### When server is found

```json
{
  "name": "Creative Update",
  "version": "0.147.3@0.9.5.5",
  "players": 0,
  "max_players": 10,
  "map": "Creative Update",
  "online": true,
  "bepinex": {
    "enabled": true,
    "mods": [
      {
        "name": "BetterUI.dll",
        "location": "/home/steam/valheim/BepInEx/plugins/BetterUI/plugins/BetterUI/BetterUI.dll"
      }
    ]
  },
  "jobs": [
    {
      "name": "AUTO_UPDATE",
      "enabled": false,
      "schedule": "*/5 * * * *"
    },
    {
      "name": "AUTO_BACKUP",
      "enabled": true,
      "schedule": "*/5 * * * *"
    },
    {
      "name": "SCHEDULED_RESTART",
      "enabled": false,
      "schedule": "0 2 * * *"
    }
  ],
  "scheduler_state": {
    "updated_at": "2026-02-16T05:00:00-08:00",
    "jobs": []
  }
}
```

### When server NOT found

```shell
[ODIN][ERROR] - Failed to request server information!
```

```json
{
  "name": "Unknown",
  "version": "Unknown",
  "players": 0,
  "max_players": 0,
  "map": "Unknown",
  "online": false,
  "bepinex": {
    "enabled": false,
    "mods": []
  },
  "jobs": [],
  "scheduler_state": {
    "updated_at": null,
    "jobs": []
  }
}
```
