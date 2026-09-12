# Webhook Configuration

## Environment Variables

| Variable                       | Default             | Required | Description                                                                                                                                                                                     |
| ------------------------------ | ------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WEBHOOK_URL                    | `<nothing>`         | FALSE    | Set this to send status notifications to your webhook or Discord endpoint. [How to create a Discord webhook URL](https://help.dashe.io/en/articles/2521940-how-to-create-a-discord-webhook-url) |
| TITLE                          | `Broadcast`         | FALSE    | Default title used by `odin notify` when no `--title` argument is provided.                                                                                                                     |
| MESSAGE                        | `Test Notification` | FALSE    | Default message used by `odin notify` when no `--message` argument is provided.                                                                                                                 |
| WEBHOOK_INCLUDE_PUBLIC_IP      | `0`                 | FALSE    | Optionally include your server's public IP in webhook notifications, useful if not using a static IP address.                                                                                   |
| PLAYER_EVENT_NOTIFICATIONS     | `0`                 | FALSE    | Set to `1` to send webhook notifications when players join or leave the server.                                                                                                                 |
| WEBHOOK_SUPPRESS_NOTIFICATIONS | `0`                 | FALSE    | Set to `1` to deliver Discord messages silently. They still appear in the channel, but do not push a notification to members. Discord only.                                                     |
| WEBHOOK_JOIN_URL               | `<nothing>`         | FALSE    | An `https://` URL to put behind a **Join Server** button on successful start notifications. Blank means no button. Discord only.                                                                |

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

## World Save Failures

Valheim keeps running when it cannot write its world (for example when the mounted save
volume is owned by another user): it logs `Error saving world!` and carries on in memory,
so the only visible symptom is a stale world after the next restart. Odin logs that line
at `ERROR` and sends a `Save Failed` notification on the first failure of a streak, then
`Save Successful` once a save goes through again. Both use the `WEBHOOK_STATUS_FAILED` /
`WEBHOOK_STATUS_SUCCESSFUL` switches. Odin also refuses to start the server at all when the
save directory is not writable, since a server that cannot save is worse than one that
does not start.

## Silencing Discord Notifications

Player join/leave events can make a busy server noisy. Setting
`WEBHOOK_SUPPRESS_NOTIFICATIONS=1` adds Discord's `SUPPRESS_NOTIFICATIONS` flag
(`4096`) to every message: it still posts to the channel, it just does not ping
anyone.

```yaml
environment:
  WEBHOOK_SUPPRESS_NOTIFICATIONS: 1
```

To silence only some events, set `flags` on that event in `discord.json`
instead of using the variable:

```json
{
  "events": {
    "player_join": {
      "content": "Notification: {{server_name}}",
      "embeds": [
        {
          "title": "{{title}}",
          "description": "{{description}}",
          "color": 3447003
        }
      ],
      "flags": 4096
    }
  }
}
```

`flags` is a bitfield, and the two mechanisms combine: with the variable set,
any flags already in the template are preserved rather than replaced. The key is
optional and omitted from the payload when unset, so existing `discord.json`
files need no change.

This applies to Discord only. Generic webhook payloads are unaffected.

## Join Server Button

Setting `WEBHOOK_JOIN_URL` adds a **Join Server** link button to the Discord
notification sent when the server finishes starting.

```yaml
environment:
  WEBHOOK_JOIN_URL: "https://valheim.example.com/connect/remote"
```

Leave it blank (the default) and no button is added.

### The URL must be https

Discord rejects a `steam://` URL on a button with a `400`, so the button cannot
link straight into Steam. It has to point at something over HTTPS that then
redirects to the `steam://` handoff.

Huginn already serves exactly that: `GET /connect/remote` responds `302` to
`steam://run/892970//+connect%20<host>:<port>`. Put Huginn behind a reverse
proxy with TLS and point `WEBHOOK_JOIN_URL` at its `/connect/remote`. Any other
redirect service works just as well -- Odin only checks that the URL parses and
uses the `https` scheme, and posts it as-is.

A plain `http://` URL is rejected with a warning and no button, because the link
is posted publicly into a channel.

### Which notification gets the button

Only a **successful start**, when the server is actually up and joinable. Stop,
failure, update and player events never carry one, since a join link on those
would be misleading.

### Why the request gains a query parameter

Webhooks that are not owned by a Discord application drop `components` unless
the request carries `with_components=true` -- Discord returns a normal success
with the button silently missing. Odin appends that parameter automatically, and
only when a button is actually attached.

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
