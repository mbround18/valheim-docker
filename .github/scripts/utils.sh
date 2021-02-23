#!/usr/bin/env bash


parse_version() {
  # Strip git ref prefix from version
  VERSION="${VERSION:-latest}"
  VERSION="${VERSION//refs\/heads\//}"
  VERSION="${VERSION//refs\/tags\/v/}"

#  VERSION=$(echo "${{ github.ref }}" | sed -e 's,.*/\(.*\),\1,')
#  # Strip "v" prefix from tag name
#  [[ "${{ github.ref }}" == "refs/tags/"* ]] && VERSION="${VERSION/refs\/tags\///}"
#  # Use Docker `latest` tag convention
  [ "${VERSION}" == "main" ] && VERSION=latest
  echo "${VERSION}"
}
