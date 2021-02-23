#!/usr/bin/env bash


parse_version() {
  # Strip git ref prefix from version
  VERSION="${VERSION:-latest}"
  # Refs starter
  VERSION="${VERSION//refs\//}"
  # Base head branches
  VERSION="${VERSION//heads\//}"
  # Clean tags
  VERSION="${VERSION//tags\/v/}"
  # Check for pull requests
  VERSION="${VERSION//pull\//}"
  # Sanitize
  VERSION="${VERSION//[\/]/-}"

  #  VERSION=$(echo "${{ github.ref }}" | sed -e 's,.*/\(.*\),\1,')
  #  # Strip "v" prefix from tag name
  #  [[ "${{ github.ref }}" == "refs/tags/"* ]] && VERSION="${VERSION/refs\/tags\///}"
  #  # Use Docker `latest` tag convention
  [ "${VERSION}" == "main" ] && VERSION=latest
  echo "${VERSION}"
}
