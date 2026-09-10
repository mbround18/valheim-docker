1. **Create a new group named `valheim`:**

```bash
sudo groupadd valheim
```

2. **Add your current user to the `valheim` group:**

```bash
sudo usermod -a -G valheim $USER
```

3. **Verify that the group was created and your user was added successfully:**

```bash
groups $USER
```

4. **Find the group ID (GID) of the `valheim` group:**

```bash
getent group valheim
```

This command will output something like `valheim:x:1001:`, where `1001` is the GID.

5. **Set the permissions for the directories to be group writable, readable, and executable:**

```bash
sudo chown -R :valheim ./saves ./server ./backups
sudo chmod -R 775 ./saves ./server ./backups
```

### 🛡️ Security: Migration to Rootless Design

To follow security best practices, this image has been moved to a **rootless design**. This means the container no longer runs as root by default. While this is a big win for security, it might require a quick tweak to your configuration to handle volume permissions correctly.

For the most reliable experience, we recommend explicitly setting the **user directive** to match your host user (usually `1000:1000`). Here is how to implement that across different platforms:

| Platform           | Implementation                                     |
| :----------------- | :------------------------------------------------- |
| **Docker Compose** | Add `user: "1000:1000"` to your service            |
| **Docker Run**     | Use the `--user 1000:1000` flag                    |
| **Kubernetes**     | Define `runAsUser: 1000` in your `securityContext` |

> **Keep the uid at `1000`.** The Valheim server only starts under a uid that has an account in the image (`1000`, or `111` for older setups), so do not replace it with your own `id -u` if that differs. Share the volumes with your host user through the group as shown above instead.

6. **Update your `docker-compose.yml` file to set the `user` directive:**

Open your `docker-compose.yml` file in a text editor and modify it as follows:

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    container_name: valheim
    user: "1000:1000" # Keep uid 1000; the server does not start under other uids
    volumes:
      - ./saves:/home/steam/.config/unity3d/IronGate/Valheim
      - ./server:/home/steam/valheim
      - ./backups:/home/steam/backups
    ports:
      - "2456-2458:2456-2458/udp"
    restart: unless-stopped
```

Keep `1000:1000` even if your host IDs are different. The Valheim server crashes at startup under a uid that has no account in the image, so your own `id -u` will not work there; the `valheim` group from the steps above is what gives your host user access to the files.

7. **Restart your Docker Compose services to apply the changes:**

```bash
docker compose down
docker compose up -d
```

This will recreate the container with the new rootless user settings.
