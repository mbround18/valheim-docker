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

> **Quick Tip:** If you aren't sure what your IDs are, run `id -u` and `id -g` on your host machine to find the correct numbers to use!

6. **Update your `docker-compose.yml` file to set the `user` directive:**

Open your `docker-compose.yml` file in a text editor and modify it as follows:

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:3
    container_name: valheim
    user: "1000:1000" # Replace with your host user/group IDs
    volumes:
      - ./saves:/home/steam/.config/unity3d/IronGate/Valheim
      - ./server:/home/steam/valheim
      - ./backups:/home/steam/backups
    ports:
      - "2456-2458:2456-2458/udp"
    restart: unless-stopped
```

Replace `1000:1000` with the output from `id -u` and `id -g` on your host if your IDs are different.

7. **Restart your Docker Compose services to apply the changes:**

```bash
docker-compose down
docker-compose up -d
```

This will recreate the container with the new rootless user settings.
