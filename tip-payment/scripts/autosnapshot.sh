#!/usr/bin/env sh
# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.
# Ex: ./autosnapshot.sh 10.1.23.506:8899 /solana/ledger/

RPC_URL=$1
LEDGER_LOCATION=$2
SNAPSHOT_DIR=/home/core/autosnapshot/
LEDGER_TOOL_PATH=/home/core/jito-solana/docker-output/solana-ledger-tool
GCLOUD_PATH=/home/core/google-cloud-sdk/bin/gcloud

fetch_last_epoch_final_slot () {
  EPOCH_INFO=$(curl -s "http://$RPC_URL" -X POST -H "Content-Type: application/json" -d '
    {"jsonrpc":"2.0","id":1, "method":"getEpochInfo"}
  ')

  if [ -z "$EPOCH_INFO" ]
  then
    echo "ERROR Unable to fetch epoch info."
    exit 1
  fi

  CURRENT_ABSOLUTE_SLOT=$(echo "$EPOCH_INFO" | jq .result.absoluteSlot)
  CURRENT_SLOT_INDEX=$(echo "$EPOCH_INFO" | jq .result.slotIndex)
  EPOCH_START_SLOT=$((CURRENT_ABSOLUTE_SLOT - CURRENT_SLOT_INDEX))
  LAST_EPOCH_END_SLOT=$((EPOCH_START_SLOT - 1))
  # Due to possible forking / disconnected blocks at the end of the last epoch
  # we check within a 40 slot range for the highest confirmed block
  RANGE_BEGIN=$((LAST_EPOCH_END_SLOT - 40))
  LAST_EPOCH_FINAL_SLOT=$(curl -s "http://$RPC_URL" -X POST -H "Content-Type: application/json" -d "
    {\"jsonrpc\": \"2.0\",\"id\":1,\"method\":\"getBlocks\",\"params\":[$RANGE_BEGIN, $LAST_EPOCH_END_SLOT]}
  " | jq '.result | last')

  if [[ "$LAST_EPOCH_FINAL_SLOT" == "null" ]]
  then
    echo "Missing block range [$RANGE_BEGIN, $LAST_EPOCH_END_SLOT] for last epoch. Nothing to do. Exiting."
    exit 1
  fi
}

create_snapshot_for_slot () {
  FOUND_SNAPSHOT=$(ls "$SNAPSHOT_DIR" | grep ".tar.zst" | grep "$1")

  if [ -z "$FOUND_SNAPSHOT" ]
  then
    echo "Didn't find snapshot for slot $1, creating..."
    RUST_LOG=info "$LEDGER_TOOL_PATH" -l $LEDGER_LOCATION create-snapshot $1
    # ledger-tool by default updates snapshots in the existing ledger directory
    # and prunes old full/incremental snapshots. copy it out to our snapshot
    # directory when finished creating.
    cp $LEDGER_LOCATION*$1* $SNAPSHOT_DIR
  else
    echo "Found snapshot, nothing to do."
  fi
}

upload_snapshot () {
  CURRENT_EPOCH=$(echo "$EPOCH_INFO" | jq .result.epoch)
  LAST_EPOCH=$((CURRENT_EPOCH - 1))
  UPLOAD_PATH="gs://jito-mainnet/$LAST_EPOCH/$FOUND_SNAPSHOT"
  SNAPSHOT_UPLOADED=$(su core -c "$GCLOUD_PATH storage ls $UPLOAD_PATH | grep $UPLOAD_PATH")

  if [ -z "$SNAPSHOT_UPLOADED" ]
  then
    echo "Snapshot not found in gcp bucket, uploading now."
    su core -c "$GCLOUD_PATH storage cp $SNAPSHOT_DIR/$FOUND_SNAPSHOT $UPLOAD_PATH"
  else
    echo "Snapshot already uploaded to gcp."
  fi
}

clean_old_snapshots () {
  ls "$SNAPSHOT_DIR"snapshot* | grep -v "$LAST_EPOCH_FINAL_SLOT" | xargs rm
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

fetch_last_epoch_final_slot
echo "last confirmed slot in previous epoch: $LAST_EPOCH_FINAL_SLOT"
create_snapshot_for_slot "$LAST_EPOCH_FINAL_SLOT"
upload_snapshot
clean_old_snapshots