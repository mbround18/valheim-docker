# Upgrading to Valheim 1.0

Valheim's 1.0 release changed two things that affect existing servers. Neither is
caused by this container, but both look like the server is broken, so they are
worth knowing about before you upgrade.

Take a backup before you start. `AUTO_BACKUP_ON_UPDATE=1` makes one for you, and
`odin backup` makes one on demand.

## Players are told they are banned

The permitted-players file changed format. Prior to 1.0, `permittedlist.txt`
contained one bare numeric ID per line:

```
76561198000000000
```

From 1.0 onward each ID must carry a `V_` prefix:

```
V_76561198000000000
```

Entries without the prefix are no longer matched, so on a server with
`PUBLIC=0` or an allowlist in use, every player — including you — is refused with
a "banned" message.

The file lives alongside your saves:

```sh
docker compose exec valheim cat /home/steam/.config/unity3d/IronGate/Valheim/permittedlist.txt
```

Add the prefix to each line and restart the server. `adminlist.txt` and
`bannedlist.txt` use the same ID format, so check those too if you maintain them.

## The server will not update: `state is 0x6 after update job`

If the update is interrupted, SteamCMD can be left with stale download
bookkeeping in the game directory. Every subsequent attempt then fails the same
way:

```
Error! App '896660' state is 0x6 after update job.
ERROR odin::server::install: steamcmd exited with code: 8
INFO  odin::server::install: No change in build version: 21981590
```

Odin detects this and recovers on its own: after a failed update it clears
SteamCMD's download state for the app and retries once. Set
`STEAMCMD_RESET_ON_FAILURE=0` to disable that behaviour.

If it still fails, clear the state by hand. This removes downloaded game files
only — your worlds live in `SAVE_LOCATION` and are not touched:

```sh
docker compose down
# Adjust the path to wherever you mounted GAME_LOCATION
sudo rm -rf ./valheim/steamapps
docker compose up -d
```

The next start re-downloads the server from scratch.

## New world content

The 1.0 content is generated at world-creation time, so an existing world will
not contain it. If you want the new content you need a new seed. To run the old
and the new world side by side, start a second container with its own `WORLD`,
`PORT`, and volume mounts — see the
[README](https://github.com/mbround18/valheim-docker/blob/main/README.md) for the
full variable list.
