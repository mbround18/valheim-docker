# Huginn

Huginn is a status server used to check the status of your Valheim server.

> [Who is Huginn?](https://en.wikipedia.org/wiki/Huginn_and_Muninn)

## Setup

1. Install Rust & git
2. Clone the repo
3. `cargo install cargo-make`
4. `makers -e production release`
5. `chmod +x ./target/debug/huginn`
6. Copy `./target/debug/huginn` to `/usr/local/bin`

## Usage

### Environment Variables

| Variable                   | Default              | Required | Description                                                                                                                                                                        |
| -------------------------- | -------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ADDRESS                    | `127.0.0.1:(PORT+1)` | FALSE    | Query address Huginn uses for server info. If unset, it defaults to loopback query port (`PORT+1`, usually `2457`). Set this if Huginn should query a different interface or host. |
| HTTP_PORT                  | `3000`               | FALSE    | HTTP port Huginn listens on. Huginn binds to `0.0.0.0` and logs `127.0.0.1` links for local convenience.                                                                           |
| HUGINN_INFO_CACHE_TTL_SECS | `2`                  | FALSE    | Status cache TTL in seconds for A2S-backed responses (`/status`, `/health`, `/readiness`, `/players`). Lower values increase freshness but also query load.                        |
| CONNECT_REMOTE_HOST        | `<unset>`            | FALSE    | Optional host/IP override for `/connect/remote`. If unset, Huginn falls back to `PUBLIC_ADDRESS`, then `ADDRESS`, then Odin public IP resolution.                                  |
| CONNECT_STEAM_APP_ID       | `892970`             | FALSE    | Steam app id used for connect deeplink generation (`steam://run/<APP_ID>//+connect%20HOST:PORT`).                                                                                  |

Note: Your server must be public (e.g., `PUBLIC=1`) for Odin+Huginn to collect and report statistics.

### Manually Launching

Simply launch `huginn` in the background with:

```shell
cd /path/to/your/valheim/server/folder
huginn &
```

### Systemd service

1. With the root user or using sudo run

   ```shell
   nano /etc/systemd/system/huginn.service
   ```

2. Copy and paste the text below

   ```toml
   [Unit]
   Description=Huginn Valheim Status Server
   After=network.target
   StartLimitIntervalSec=0

   [Service]
   Type=simple
   Restart=always
   RestartSec=1
   User=steam
   Environment="HTTP_PORT=3000" "ADDRESS=127.0.0.1:2457"
   WorkingDirectory=/home/steam/valheim
   ExecStart=/usr/bin/env /usr/local/bin/huginn

   [Install]
   WantedBy=multi-user.target
   ```

3. Make any necessary changes to the service to fit your needs.
   (Remember, the port you use in your `ADDRESS` must be your query port which is +1 of your game port.)

4. Next save the file and start the service.

   ```shell
   sudo systemctl start huginn
   ```

5. To have the server start on server launch, run:

   ```shell
   sudo systemctl enable huginn
   ```

## Endpoints

| Endpoint          | Description                                                                                                                                                                             |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/metrics`        | Provides Prometheus-compatible server status output. [Guide to setting up a dashboard](https://github.com/mbround18/valheim-docker/discussions/330).                                      |
| `/status`         | Provides a more traditional JSON output of the server status.                                                                                                                           |
| `/connect/local`  | Redirect to `steam://run/892970//+connect%20127.0.0.1:PORT`. Browser CORS fetch clients receive JSON `{ steam_url, host, port, redirect }` for compatibility.                           |
| `/connect/remote` | Redirect to `steam://run/892970//+connect%20<public host>:PORT`. Browser CORS fetch clients receive JSON `{ steam_url, host, port, redirect }` for compatibility.                       |
| `/health`         | Returns `200` when the server is online, `503` when offline.                                                                                                                            |
| `/readiness`      | Kubernetes readiness probe; `200` only if the server is online.                                                                                                                         |
| `/liveness`       | Kubernetes liveness probe; returns `200` when Huginn is alive.                                                                                                                          |
| `/mods`           | Returns installed mod metadata.                                                                                                                                                         |
| `/players`        | Returns player counts and player names when available.                                                                                                                                  |
| `/metadata`       | Returns safe Odin/Huginn runtime metadata (ports, build id, beta mode, scheduler toggles/schedules, cache/runtime info).                                                                |
| `/openapi.json`   | OpenAPI specification for the HTTP API.                                                                                                                                                 |
| `/docs`           | Swagger UI for interactive API documentation.                                                                                                                                           |
