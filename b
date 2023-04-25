#!/usr/bin/env bash
# Builds jito-programs in a docker container.
# Useful for running on machines that might not have cargo installed but can run docker (Flatcar Linux).
# run `./f true` to compile with debug flags

set -eux

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

DOCKER_BUILDKIT=1 docker build \
  -t jitolabs/jito-programs \
  . --progress=plain

# Creates a temporary container, copies binary built inside container there and
# removes the temporary container.
docker rm temp || true
docker container create --name temp jitolabs/jito-programs
mkdir -p $SCRIPT_DIR/container-out

# Outputs the binary to the host machine
docker container cp temp:/jito-programs/container-out $SCRIPT_DIR/
docker rm temp
