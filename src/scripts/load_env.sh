#!/usr/bin/env sh

echo "
Loading ENV.....
"

export "$(grep ^PORT= /home/steam/.env)"
export "$(grep ^NAME= /home/steam/.env)"
export "$(grep ^WORLD= /home/steam/.env)"
export "$(grep ^PUBLIC= /home/steam/.env)"
export "$(grep ^PASSWORD= /home/steam/.env)"
export "$(grep ^AUTO_UPDATE= /home/steam/.env)"

echo "
Variables loaded.....

Port: ${PORT}
Name: ${NAME}
World: ${WORLD}
Public: ${PUBLIC}
Password: (REDACTED)
Auto Update: ${AUTO_UPDATE}
"
