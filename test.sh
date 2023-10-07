#!/usr/bin/env bash



export MODIFIERS="
raids=muchmore
combat=hard
deathpenalty=casual
"

echo "${MODIFIERS}" | xargs echo -n | tr ' ' ',' | sed 's/,,/,/g'