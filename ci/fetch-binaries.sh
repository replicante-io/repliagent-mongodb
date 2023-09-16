#!/bin/bash
set -e

REPO_URL="docker.io/replicanteio/repliagent-mongodb"
PLATFORMS="linux/amd64 linux/arm64"
TARGET_DIR="target/prebuilt"

help() {
  echo "Usage ./fetch-binaries <VERSION_TAG>"
}

# --- Parse CLI --- #
if [ "$#" -ne 1 ]; then
  help
  exit 1
fi

case "$1" in
  -h|--help)
    help
    exit 0
    ;;
  *)
    VERSION=$1
    ;;
esac

# --- Helpers --- #
fetch() {
  platform=$1
  cid=$(podman run --rm -d --platform "${platform}" "${REPO_URL}:${VERSION}" sleep 30)
  podman cp "${cid}:/opt/replicante/bin/repliagent-mongodb" "${TARGET_DIR}/repliagent-mongodb-${platform//\//-}"
}

# --- Main --- #
if [ -e "${TARGET_DIR}" ]; then
  echo "Clearing ${TARGET_DIR} ..."
  rm -r "${TARGET_DIR}"
fi
mkdir -p "${TARGET_DIR}"

for platform in $(echo ${PLATFORMS}); do
  echo "Fetching binary for platform ${platform} ..."
  fetch "${platform}"
done

echo "Generating SHA256 checksums for binaries ..."
pushd "${TARGET_DIR}" > /dev/null
sha256sum * > checksums.txt
popd > /dev/null
