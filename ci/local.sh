#!/bin/bash
set -e

for_version() {
  version="$1"
  full_mode=""
  if [ "${version}" == "stable" ]; then
    full_mode="--full"
  fi

  echo "Clean up workspaces for version ${version}"
  rustup run "${version}" cargo clean

  echo "Run CI for version ${version}"
  rustup run "${version}" ci/check-workspace.sh ${full_mode} Agent Cargo.toml
}

for_version "stable"
for_version "1.68.0"
for_version "nightly"
