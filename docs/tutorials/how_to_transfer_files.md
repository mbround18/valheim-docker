# How To Transfer Files

> For this tutorial, we will be using [schollz/croc] which is a cli tool for sending files. This is for temporary access to transfer files and if you want to do this frequently, then consider running a [file browser](https://hub.docker.com/r/hurlenko/filebrowser) sidecar container.

## Setup

1. Access the container via the root user.

```sh
# This can be accomplished via many methods but for this tutorial we will be using docker-compose.
docker-compose exec valheim bash

# 1. Replace valheim with the name of your container.
# 2. This tutorial cannot account fo all the wide variety of names or methods on which the container is created.
# 3. You just need to access the container via the root user to install croc.
```

1. Run `curl https://getcroc.schollz.com | bash` inside the `mbround18/valheim` container.
2. Run `curl https://getcroc.schollz.com | bash` inside the destination machine. (If the destination machine is not a linux|unix based system with access to bash, please look at [schollz/croc] repo for installation instructions.)

![Install Croc Success](../assets/transfer_file_demo/install_croc.png)

## Instructions

> For this example, we will be transfering world files from one instance of `mbround18/valheim` to another `mbround18/valheim`. If you have a slightly different use case, such as transfering backups or something else, then this guide will be helpful but not match 1 to 1.

1. Access the `mbround18/valheim` container A and container B as the steam user.

   ```sh
   # This can be accomplished via many methods but for this tutorial we will be using docker-compose.
   docker-compose exec --user steam valheim bash

   # Replace valheim with the name of your container.
   # This tutorial cannot account fo all the wide variety of names or methods on which the container is created.
   # You just need to access the container via the steam user when sending the files.
   ```

2. Stop the valheim server on both machines, `odin stop`
3. Cd into your saves dir with `cd /home/steam/.config/unity3d/IronGate/Valheim` on both container A and container B.
4. With croc already installed, run `croc send ./*` on container A.

   ![croc send command](../assets/transfer_file_demo/send_croc.png)

5. Once it spits out the transfer key, copy the transfer key to container B and hit the enter key.

   ![croc send command success](../assets/transfer_file_demo/send_croc_success.png)

6. You can now safely shutdown container A and restart container B.

[schollz/croc]: https://github.com/schollz/croc
