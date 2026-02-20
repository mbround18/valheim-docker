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

If the values are not `1000:1000`, use your real values in `user: "UID:GID"` and `--user UID:GID`.

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
