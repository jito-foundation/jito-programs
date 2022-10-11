#!/usr/bin/env sh

DIR="$( cd "$( dirname "$0" )" && pwd )"
source ./${DIR}/utils.sh

# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.
# Ex: ./autosnapshot.sh 10.1.23.506:8899 /solana/ledger/

set -e

RPC_URL=$RPC_URL
LEDGER_LOCATION=$LEDGER_LOCATION
SNAPSHOT_DIR=$SNAPSHOT_DIR
LEDGER_TOOL_PATH=$LEDGER_TOOL_PATH
HOST_NAME=$HOST_NAME
ENVIRONMENT=$ENVIRONMENT

create_snapshot_for_slot() {
  local snapshot_slot=$1

  # shellcheck disable=SC2012
  local snapshot_file=$(ls "$SNAPSHOT_DIR" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
  if [ -z "$snapshot_file" ]
  then
    echo "Didn't find snapshot for slot $1, creating..."
    RUST_LOG=info "$LEDGER_TOOL_PATH" -l "$LEDGER_LOCATION" create-snapshot "$snapshot_slot"
    # ledger-tool by default updates snapshots in the existing ledger directory
    # and prunes old full/incremental snapshots. copy it out to our snapshot
    # directory when finished creating.
    cp "$LEDGER_LOCATION"*"$snapshot_slot"* $SNAPSHOT_DIR
  else
    echo "Found snapshot, nothing to do."
  fi

  # shellcheck disable=SC2012
  local snapshot_file=$(ls "$SNAPSHOT_DIR" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
  echo "$snapshot_file"
}

upload_snapshot() {
  local epoch_info=$1
  local snapshot_file=$2

  local current_epoch=$(echo "$epoch_info" | jq .result.epoch)
  local last_epoch=$((current_epoch - 1))
  local upload_path="gs://jito-$ENVIRONMENT/$last_epoch/$HOST_NAME/$snapshot_file"
  local snapshot_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$snapshot_uploaded" ]
  then
    echo "Snapshot not found in gcp bucket, uploading now."
    echo "snapshot_file: $snapshot_file"
    echo "upload_path: $upload_path"
    gcloud storage cp $SNAPSHOT_DIR/"$snapshot_file" "$upload_path"
  else
    echo "Snapshot already uploaded to gcp."
  fi
}

rm_snapshot_file() {
  local snapshot_slot=$1

  # shellcheck disable=SC2012
  maybe_snapshot=$(ls "$SNAPSHOT_DIR"snapshot* | { grep -E "$snapshot_slot" || true; })
  if [ -z "$maybe_snapshot" ]
  then
    echo "Snapshot ${maybe_snapshot} not found, exiting."
    exit 1
  else
    rm "$maybe_snapshot"
  fi
}

if [ -z "$RPC_URL" ]
then
    echo "Please pass rpc url as first parameter to autosnapshot"
    exit
fi

if [ -z "$LEDGER_LOCATION" ]
then
  echo "Please pass ledger path as second parameter to autosnapshot"
  exit
fi

epoch_info=$(fetch_epoch_info "$RPC_URL" | tail -n 1)
epoch_final_slot=$(calculate_epoch_end_slot "$epoch_info" | tail -n 1)
echo "last confirmed slot in previous epoch: $epoch_final_slot"
snapshot_file=$(create_snapshot_for_slot "$epoch_final_slot" | tail -n 1)
upload_snapshot "$epoch_info" "$snapshot_file"
rm_snapshot_file "$epoch_final_slot"
