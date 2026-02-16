# Webhook Configuration

## Environment Variables

| Variable                  | Default             | Required | Description                                                                                                                                                                                                                                   |
| ------------------------- | ------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WEBHOOK_URL               | `<nothing>`         | FALSE    | Set this to send status notifications to your webhook or Discord endpoint. [How to create a Discord webhook URL](https://help.dashe.io/en/articles/2521940-how-to-create-a-discord-webhook-url)                                           |
| TITLE                     | `Broadcast`         | FALSE    | Default title used by `odin notify` when no `--title` argument is provided.                                                                                                                                                                   |
| MESSAGE                   | `Test Notification` | FALSE    | Default message used by `odin notify` when no `--message` argument is provided.                                                                                                                                                               |
| WEBHOOK_INCLUDE_PUBLIC_IP | `0`                 | FALSE    | Optionally include your server's public IP in webhook notifications, useful if not using a static IP address.                                                                                                                                |
| PLAYER_EVENT_NOTIFICATIONS | `0`                | FALSE    | Set to `1` to send webhook notifications when players join or leave the server.                                                                                                                                                               |

## POST Body Example

```json
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

- The expected HTTP status codes returned from the webhook should be either 204 or 201 to be considered successful.
  - 204 is the default return HTTP code for a webhook, meaning the request has been processed.
  - 201 was included in case you want to stream into an endpoint for creating a resource.
    - Example 1, logging actions on the server.
    - Example 2, using json-server to debug webhooks.

## Discord Configs

Generates a file in the server directory called `discord.json`. There are a series of variables provided that you can use
from the templating engine. If you use a variable like `{{some_var}}` that is not provided, it renders as blank.
If values are blank, Discord may reject the payload.

     title: String::from(&notification.event_type.name),
      description: String::from(&notification.event_message),
      status: String::from(&notification.event_type.status),
      timestamp: String::from(&notification.timestamp),
      server_name: get_server_name(),

| Variable          | Value                          | Example                               |
| ----------------- | ------------------------------ | ------------------------------------- |
| `{{title}}`       | Event title                    | `Start`                               |
| `{{description}}` | Event Message                  | `Server Status: Start Successful`     |
| `{{status}}`      | Event Status                   | `Successful`                          |
| `{{timestamp}}`   | Timestamp of event             | `2021-05-30T08:16:39.294366700-07:00` |
| `{{server_name}}` | Name pulled from env or config | `Created with Valheim Docker`         |

## Developing/Debugging Webhooks

1. Start json-server

   ```shell
   docker run --rm -p 3000:3000 vimagick/json-server  -H 0.0.0.0 -p 3000 -w db.json
   ```

2. Run notify against the webhook

   ```shell
   cargo run -- notify "Testing webhook notification" --webhook "http://127.0.0.1:3000/posts"
   ```
