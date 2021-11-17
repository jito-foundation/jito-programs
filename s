#!/usr/bin/env sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

if [ "$USER" = "lucasbruder" ]; then
  HOST=jito@100.100.182.81
else
  HOST=jito@94.130.200.9
fi

echo "Syncing to host: $HOST"

rsync -avh --delete --exclude=".anchor" --exclude="test-ledger" --exclude=".git" --exclude="target" --exclude="node_modules" $SCRIPT_DIR $HOST:~/
