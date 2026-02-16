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

6. **Update your `docker-compose.yml` file to set the PGID:**

Open your `docker-compose.yml` file in a text editor and modify it as follows:

```yaml
version: "3"
services:
  valheim:
    image: mbround18/valheim:3
    container_name: valheim
    user: "111:1001" # Replace with the actual GID from step 4
    volumes:
      - ./saves:/home/steam/.config/unity3d/IronGate/Valheim
      - ./server:/home/steam/valheim
      - ./backups:/home/steam/backups
    ports:
      - "2456-2458:2456-2458/udp"
    restart: unless-stopped
```

Replace `1001` with the actual GID you found in step 4. You can also dynamically set the PUID using the `id -u` command to get your current user ID.

7. **Restart your Docker Compose services to apply the changes:**

```bash
docker-compose down
docker-compose up -d
```

This will recreate the container with the new PGID settings.
