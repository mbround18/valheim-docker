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

| Variable  | Default               | Required | Description                                                                                                                  |
| --------- | --------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------- |
| ADDRESS   | `Your Public IP`      | FALSE    | This setting is used in conjunction with `odin status` and setting this will stop `odin` from trying to fetch your public IP |
| HTTP_PORT | `anything above 1024` | FALSE    | Setting this will spin up a little http server that provides two endpoints for you to call.                                  |


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

| Endpoint   | Description |
|------------|------------|
| `/metrics` | Provides a Prometheus compatible output of the server status. [Click here to see a guide on how to get a dashboard setup.](https://github.com/mbround18/valheim-docker/discussions/330) | 
| `/status`  | Provides a more traditional JSON output of the server status. |
