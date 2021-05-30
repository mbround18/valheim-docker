# Webhook Configuration

## Environment Variables

| Variable                          | Default                            | Required | Description                                                                                                                                                                                                                                   |
| --------------------------------- | ---------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WEBHOOK_URL                       | `<nothing>`                        | FALSE    | Supply this to get information regarding your server's status in a webhook or Discord notification! [Click here to learn how to get a webhook url for Discord](https://help.dashe.io/en/articles/2521940-how-to-create-a-discord-webhook-url) |
| WEBHOOK_BROADCAST_MESSAGE         | CHANGE_ME                          | TRUE     | You set this. See `odin notify --help`                                                                                                                                                                                                        |
| WEBHOOK_STATUS_RUNNING            | "0"                                | FALSE    | Posts a running status to discord when a command is initialized. |
| WEBHOOK_STATUS_FAILED             | "1"                                | FALSE    | Posts a failed status to discord in the event of a failure. |
| WEBHOOK_STATUS_SUCCESSFUL         | "1"                                | FALSE    | Posts a running status to discord when the command succeeds. |

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

| Key                 | Description                 |
| ------------------- | --------------------------- |
| `event_type.name`   | Name of the event           |
| `event_type.status` | Status of the event         |
| `event_message`     | A description of the event. |
| `timestamp`         | ISO8601 timestamp           |

## Considerations

- The expected HTTP codes returned from the webhook should be either 204 or 201 to be considered successful.
  - 204 is the default return http code for a webhook as it signifies the request has been processed.
  - 201 was included in case you want to stream into an endpoint for creating a resource.
    - Example 1, logging actions on the server.
    - Example 2, using json-server to debug webhooks.
  
## Discord Configs

Generates a file in the server directory called `discord.json`. There are a series of variables provided that you can use 
from the templating engine. See below, note if you use `{{some_var}}` and its not provided by the table below it will show as a blank.
If any of the values turn out blank, discord might reject the post.

     title: String::from(&notification.event_type.name),
      description: String::from(&notification.event_message),
      status: String::from(&notification.event_type.status),
      timestamp: String::from(&notification.timestamp),
      server_name: get_server_name(),

| Variable | Value | Example |
|----------|-------|-------------|
| `{{title}}` | Event title | `Start`
| `{{description}}` | Event Message | `Server Status: Start Successful` |
| `{{status}}` | Event Status | `Successful`
| `{{timestamp}}` | tiemstamp of event | `2021-05-30T08:16:39.294366700-07:00` |
| `{{server_name}}` | Name pulled from env or config | `Created with Valheim Docker` |  

## Developing/Debugging Webhooks

1. Start json-server

   ```shell
   docker run --rm -p 3000:3000 vimagick/json-server  -H 0.0.0.0 -p 3000 -w db.json
   ```

2. Run notify against the webhook

   ```shell
   cargo run -- notify "Derp Testing another notification" --webhook "http://127.0.0.1:3000/posts"
   ```
