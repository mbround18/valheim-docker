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
```

With `--json`
(with ValheimPlus for example)
```json
{
  "name":"Fancy Name",
  "version":"0.147.3@0.9.5.5",
  "players":0,
  "max_players":10,
  "map":"Fancy Name",
  "online":true,
  "bepinex":{
    "enabled":true,
    "mods":[
      {
        "name":"ValheimPlus.dll",
        "location":"/home/steam/valheim/BepInEx/plugins/ValheimPlus.dll"
      }
    ]
  },
  "jobs":[
    {
      "name":"AUTO_UPDATE",
      "enabled":false,
      "schedule":"*/5 * * * *"
    },
    {
      "name":"AUTO_BACKUP",
      "enabled":true,
      "schedule":"*/5 * * * *"
    }
  ]
}
```

## 🆕 HTTP server for serving status

Setting the `HTTP_PORT` variable to any number will spin up a small http server that can pull server status.

You can access it via `http://127.0.0.1:HTTP_PORT/status`.
You are responsible for putting your status endpoint behind SSL or authentication if you desire.
