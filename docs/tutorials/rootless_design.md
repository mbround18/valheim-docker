# Rootless Design Guide

This project uses a rootless container design to reduce security risk. The container does not run as root by default, and you should set an explicit user so mounted volumes have correct ownership.

## Why Rootless Matters

- Reduces the blast radius if a process is compromised
- Avoids writing files as `root` on the host
- Aligns with container security best practices

## Recommended Configuration

Use your host user/group IDs (most commonly `1000:1000`).

### Docker Compose

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    user: "1000:1000"
```

### Docker Run

```bash
docker run --user 1000:1000 mbround18/valheim:3
```

### Kubernetes

```yaml
securityContext:
  runAsUser: 1000
```

## Find Your IDs

Run these commands on your host:

```bash
id -u
id -g
```

If the values are not `1000:1000`, still run the container as `1000:1000`: the server only
starts under a uid that has an account in the image (see
[below](#upgrading-from-images-where-steam-was-uid-111)). Give your host user access to the
volumes through the group instead; see [Group File System Access](./group_file_system_access.md).

## Migration Checklist

1. Stop the running container.
2. Update your compose/run config with explicit `user`.
3. Ensure volume paths are owned by that user/group.
4. Start the container again.

Example permission fix:

```bash
sudo chown -R 1000:1000 ./valheim
```

Adjust `1000:1000` and paths to match your environment.

## Upgrading From Images Where `steam` Was uid 111

Older images created the `steam` user as uid `111`, so the documented `user: "1000:1000"`
could not write the image's own home directory and startup failed with
`Preflight write check failed`. `steam` is now uid `1000`, matching the examples above.
What that means for an existing server:

| Your setup                                        | What to do                                                                                                                                                                  |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No `user:` set                                    | Nothing. On first start the container re-owns its volumes as `1000:1000`, so host files owned by uid `111` become owned by uid `1000`.                                      |
| `user: "1000:1000"` or `runAsUser: 1000`          | This now works as documented. If you had chowned your volumes to `111`, chown them to `1000` once (below).                                                                  |
| `user: "111:1000"` or `runAsUser: 111`            | Keeps working: the image keeps a `steam-legacy` account on uid `111` in group `1000`. Move to `1000:1000` when convenient: stop the server, chown (below), switch the user. |
| Kubernetes with `allowPrivilegeEscalation: false` | The container cannot re-own volumes by itself. Set `fsGroup: 1000` in the pod `securityContext`, or chown the volume once (below).                                          |

```bash
sudo chown -R 1000:1000 ./valheim ./saves ./backups
```

> **Only uids `1000` and `111` can run the server.** The Valheim server crashes at
> startup (a segfault in its PlayFab logger) when the container's uid has no account in
> the image, so a `user:` such as `1001:1001` fails even when every volume is writable.
> If your host user is not `1000`, keep `user: "1000:1000"` and give your host user
> access to the volumes through the group instead; see
> [Group File System Access](./group_file_system_access.md).
