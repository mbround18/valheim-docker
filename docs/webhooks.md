# Webhook Configuration

## Environment Variables

| Variable                            | Default                            | Required | Description |
|-------------------------------------|------------------------------------|----------|-------------|
| WEBHOOK_URL                         | ``                                 | FALSE    | Supply this to get information regarding your server's status in a webhook or Discord notification! [Click here to learn how to get a webhook url for Discord](https://help.dashe.io/en/articles/2521940-how-to-create-a-discord-webhook-url) |
| WEBHOOK_BROADCAST_MESSAGE           | CHANGE_ME                          | TRUE     | You set this. See `odin notify --help` |
| WEBHOOK_UPDATING_MESSAGE            | `Server Status: Updating`          | FALSE    | Set the Updating message of your server |
| WEBHOOK_UPDATE_SUCCESSFUL_MESSAGE   | `Server Status: Update Successful` | FALSE    | Set the Update Successful message of your server |
| WEBHOOK_UPDATE_FAILED_MESSAGE       | `Server Status: Update Failed`     | FALSE    | Set the Update Failed message of your server |
| WEBHOOK_STARTING_MESSAGE            | `Server Status: Starting`          | FALSE    | Set the Starting message of your server |
| WEBHOOK_START_SUCCESSFUL_MESSAGE    | `Server Status: Start Successful`  | FALSE    | Set the Start Successful message of your server |
| WEBHOOK_START_FAILED_MESSAGE        | `Server Status: Start Failed`      | FALSE    | Set the Start Failed message of your server |
| WEBHOOK_STOPPING_MESSAGE            | `Server Status: Stopping`          | FALSE    | Set the Stopping message of your server |
| WEBHOOK_STOP_SUCCESSFUL_MESSAGE     | `Server Status: Stop Successful`   | FALSE    | Set the Stop Successful message of your server |
| WEBHOOK_STOP_FAILED_MESSAGE         | `Server Status: Stop Failed`       | FALSE    | Set the Stop Failed message of your server |


## POST Body Example

```Json
{
  "event_type": {
    "name": "Broadcast",
    "status": "triggered"
  },
  "event_message": "Server Status: Broadcast",
  "timestamp": "02/22/2021 17:18:04 -08:00"
}
```

| Key             | Description |
|-----------------|-------------|
| `event_type.name`  | Name of the event |
| `event_type.status`  | Status of the event |
| `event_message` | A description of the event. |
| `timestamp`     | ISO8601 timestamp |

## Considerations

- The expected HTTP codes returned from the webhook should be either 204 or 201 to be considered successful. 
    - 204 is the default return http code for a webhook as it signifies the request has been processed.
    - 201 was included in case you want to stream into an endpoint for creating a resource. 
      - Example 1, logging actions on the server.
      - Example 2, using json-server to debug webhooks.  
    
## Developing/Debugging Webhooks

1. Start json-server
   
    ```shell
    docker run --rm -p 3000:3000 vimagick/json-server  -H 0.0.0.0 -p 3000 -w db.json
    ```
   
2. Run notify against the webhook
   
    ```shell
    cargo run -- notify "Derp Testing another notification" --webhook "http://127.0.0.1:3000/posts"
    ```


