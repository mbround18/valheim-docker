# Restoring a Valheim Server

## Step 1: Stop the Valheim Server

Before making any changes to the server files, you need to stop the Valheim server. This ensures that no data gets corrupted during the process.

### How to Stop the Valheim Server

1. **Access the Server Console:**
   - If you're using a hosting service, log in to your control panel and find the console or command-line interface.
   - If you're hosting the server locally, open the terminal or command prompt on the machine running the server.

2. **Stop the Server Using Docker Compose:**
   - Enter one of the following commands to stop the Valheim server:
     ```sh
     docker compose down
     ```
     or
     ```sh
     docker compose exec valheim odin stop
     ```
   - Wait until you see a confirmation message indicating the server has stopped.

## Step 2: Delete the Save Files

Now that the server is stopped, you can delete the existing save files.

### How to Delete Save Files

1. **Navigate to the Saves Directory:**
   - The saves directory is mapped to a local path using Docker volumes. Based on your `docker-compose.yml`, the path is:
     ```
     /path/to/saves
     ```

2. **Delete the Files:**
   - Locate the files in the `/path/to/saves` folder and delete them. You can do this via the command line:
     ```sh
     rm -rf /path/to/saves/*
     ```

## Step 3: Untar the Desired Tar File into the Saves Folder

You need to extract the desired save file tar archive into the saves folder.

### How to Untar the File

1. **Navigate to the Directory Containing the Tar File:**
   - Use the terminal or command prompt to navigate to the directory where your tar file is located.

2. **Untar the File:**
   - Enter the following command to untar the file into the saves folder:
     ```sh
     tar -xvf <your-tar-file>.tar -C /path/to/saves
     ```
   - Replace `<your-tar-file>.tar` with the name of your tar file. For example:
     ```sh
     tar -xvf backup.tar -C /path/to/saves
     ```

## Step 4: Start the Valheim Server

Now that the new save files are in place, you can start the Valheim server again.

### How to Start the Valheim Server

1. **Start the Server Using Docker Compose:**
   - Enter the following command to start the Valheim server:
     ```sh
     docker compose up -d
     ```
   - Wait until the server is up and running. You should see confirmation messages in the console.

## Docker Compose File for Reference

Here's the `docker-compose.yml` file for reference:

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    user: "1000:1000"
    environment:
      PORT: 2456
      NAME: "Created With Valheim Docker"
      PASSWORD: "Change Me! Please."
      TZ: "America/Los_Angeles"
      AUTO_UPDATE: 1
      UPDATE_ON_STARTUP: 0
      AUTO_UPDATE_SCHEDULE: "0 1 * * *"
      TYPE: Vanilla
    build:
      context: .
      dockerfile: ./Dockerfile.valheim
    ports:
      - "2456:2456/udp"
      - "2457:2457/udp"
      - "2458:2458/udp"
    volumes:
      - /path/to/saves:/home/steam/.config/unity3d/IronGate/Valheim
      - /path/to/backups:/home/steam/backups
      - odin-output:/home/steam/.odin
```
