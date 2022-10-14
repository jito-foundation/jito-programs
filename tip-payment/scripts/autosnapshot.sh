#!/usr/bin/env sh
# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
source ./${DIR}/utils.sh

RPC_URL=$1
HOST_NAME=$2

# make sure all env vars are set for this script
check_env() {
  if [ -z "$RPC_URL" ]; then
    echo "Must pass RPC URL as first arg"
    exit 1
  fi

  if [ -z "$HOST_NAME" ]; then
    echo "Must pass host name as second arg"
    exit 1
  fi

  if [ -z "$LEDGER_LOCATION" ]; then
    echo "LEDGER_LOCATION must be set"
    exit 1
  fi

  if [ -z "$SNAPSHOT_DIR" ]; then
    echo "SNAPSHOT_DIR must be set"
    exit 1
  fi

  if [ -z "$SOLANA_CLUSTER" ]; then
    echo "SOLANA_CLUSTER must be set"
    exit 1
  fi
}

create_snapshot_for_slot() {
  local snapshot_slot=$1

  # shellcheck disable=SC2012
  local snapshot_file=$(ls "$SNAPSHOT_DIR" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
  if [ -z "$snapshot_file" ]; then
    clean_old_snapshot_files
    echo "Didn't find snapshot for slot $snapshot_slot, creating..."
    RUST_LOG=info solana-ledger-tool -l "$LEDGER_LOCATION" create-snapshot "$snapshot_slot"
    # ledger-tool by default updates snapshots in the existing ledger directory
    # and prunes old full/incremental snapshots. copy it out to our snapshot
    # directory when finished creating.
    cp "$LEDGER_LOCATION"*"$snapshot_slot"* "$SNAPSHOT_DIR"
  else
    echo "Found snapshot, nothing to do."
  fi

  # shellcheck disable=SC2012
  SNAPSHOT_FILE=$(ls "$SNAPSHOT_DIR" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
}

upload_snapshot() {
  local epoch_info=$1
  local snapshot_file=$2

  local current_epoch=$(echo "$epoch_info" | jq .result.epoch)
  local last_epoch=$((current_epoch - 1))
  local upload_path="gs://jito-$SOLANA_CLUSTER/$last_epoch/$HOST_NAME/$snapshot_file"
  local snapshot_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$snapshot_uploaded" ]; then
    echo "Snapshot not found in gcp bucket, uploading now."
    echo "snapshot_file: $snapshot_file"
    echo "upload_path: $upload_path"
    gcloud storage cp $SNAPSHOT_DIR/"$snapshot_file" "$upload_path"
  else
    echo "Snapshot already uploaded to gcp."
  fi
}

clean_old_snapshot_files() {
  # shellcheck disable=SC2012
  maybe_snapshot=$(ls "$SNAPSHOT_DIR"snapshot* 2>/dev/null | { grep -E "snapshot" || true; })
  if [ -z "$maybe_snapshot" ]; then
    echo "No snapshots to clean up."
  else
    rm "$maybe_snapshot"
  fi
}

check_env

fetch_epoch_info "$RPC_URL"
calculate_epoch_end_slot "$EPOCH_INFO"
echo "last confirmed slot in previous epoch: $EPOCH_FINAL_SLOT"
create_snapshot_for_slot "$EPOCH_FINAL_SLOT"
upload_snapshot "$EPOCH_INFO" "$SNAPSHOT_FILE"
