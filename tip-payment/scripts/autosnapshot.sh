#!/usr/bin/env bash
# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.
# After creating a snapshot, it creates a snapshot of metadata
# and merkle roots before uploading them on-chain
# NOTE: this file depends on binaries being built in jito-solana

# error out, unset variables are errors, and echo commands
set -eux

RPC_URL=$1

post_slack_message() {
  local bearer=$1
  local channel=$2
  local msg=$3

  local hostname

  hostname=$(hostname)

  echo $msg

  curl -d "text=$hostname: $msg" -d "channel=$channel" -H "Authorization: Bearer $bearer" -X POST https://slack.com/api/chat.postMessage
}

# make sure all env vars are set for this script
check_env_vars_set() {
  if [ -z "$RPC_URL" ]; then
    echo "RPC_URL must be set"
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

  if [ -z "$TIP_DISTRIBUTION_PROGRAM_ID" ]; then
    echo "TIP_DISTRIBUTION_PROGRAM_ID must be set"
    exit 1
  fi

  if [ -z "$TIP_PAYMENT_PROGRAM_ID" ]; then
    echo "TIP_PAYMENT_PROGRAM_ID must be set"
    exit 1
  fi

  if [ -z "$KEYPAIR" ]; then
    echo "KEYPAIR must be set"
    exit 1
  fi

  if [ -z "$LEDGER_DIR" ]; then
    echo "LEDGER_DIR must be set"
    exit 1
  fi

  if [ -z "$SLACK_APP_TOKEN" ]; then
    echo "SLACK_APP_TOKEN must be set"
    exit 1
  fi

  if [ -z "$SLACK_CHANNEL" ]; then
    echo "SLACK_CHANNEL must be set"
    exit 1
  fi
}

# read epoch info from RPC endpoint
get_epoch_info() {
  local rpc_url=$1

  local epoch_info

  epoch_info=$(curl "$rpc_url" -X POST -H "Content-Type: application/json" -d '
      {"jsonrpc":"2.0","id":1, "method":"getEpochInfo"}
    ')
  if [ -z "$epoch_info" ]; then
    echo "ERROR Unable to fetch epoch info."
    exit 1
  fi
  echo "$epoch_info"
}

# gets the snapshot filename, assuming it's present
# cant be known ahead of time because the snapshot filename includes a hash
get_snapshot_filename() {
  local snapshot_dir=$1
  local snapshot_slot=$2

  local snapshot_file

  snapshot_file=$(ls "$snapshot_dir" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
  echo "$snapshot_file"
}

# creates a snapshot for the given slot and returns the filename
create_snapshot_for_slot() {
  local snapshot_slot=$1
  local snapshot_dir=$2
  local ledger_location=$3

  local snapshot_file
  local exit_status

  # produces snapshot in $snapshot_dir
  # shellcheck disable=SC2012
  RUST_LOG=info solana-ledger-tool -l "$ledger_location" create-snapshot "$snapshot_slot" "$snapshot_dir"

  # TODO: figure this out
  # for some reason solana-ledger-tool error doesn't cause this script to exit out, so check status here
  exit_status=$?
  if [ $exit_status -ne 0 ]; then
      echo "solana-ledger-tool returned $exit_status"
      exit $exit_status
  fi

  # snapshot file should exist now, grab the filename here (snapshot-$slot-$hash.tar.zst)
  snapshot_file=$(get_snapshot_filename "$snapshot_dir" "$snapshot_slot")

  echo "$snapshot_file"
}

generate_stake_meta() {
  local slot=$1
  local snapshot_dir=$2
  local stake_meta_filename=$3
  local tip_distribution_program_id=$4
  local tip_payment_program_id=$5

  rm -rf "$snapshot_dir"/stake-meta.accounts || true
  rm -rf "$snapshot_dir"/tmp* || true
  rm -r /tmp/.tmp* || true # calculate hash stuff stored here

  RUST_LOG=info solana-stake-meta-generator \
    --ledger-path "$snapshot_dir" \
    --tip-distribution-program-id "$tip_distribution_program_id" \
    --out-path "$snapshot_dir/$stake_meta_filename" \
    --snapshot-slot "$slot" \
    --tip-payment-program-id "$tip_payment_program_id"

  rm -rf "$snapshot_dir"/stake-meta.accounts || true
  rm -rf "$snapshot_dir"/tmp* || true
  rm -r /tmp/.tmp* || true # calculate hash stuff stored here
}

generate_merkle_trees() {
  local stake_meta_filepath=$1
  local merkle_tree_filepath=$2
  RUST_LOG=info solana-merkle-root-generator \
    --stake-meta-coll-path "$stake_meta_filepath" \
    --out-path "$merkle_tree_filepath"
}

claim_tips() {
  local merkle_trees_path=$1
  local rpc_url=$2
  local tip_distribution_program_id=$3
  local keypair_path=$4

  RUST_LOG=info solana-claim-mev-tips \
    --merkle-trees-path "$merkle_trees_path" \
    --rpc-url "$rpc_url" \
    --tip-distribution-program-id "$tip_distribution_program_id" \
    --keypair-path "$keypair_path"
}

get_gcloud_path() {
  local solana_cluster=$1
  local epoch=$2
  local file_name=$3

  local upload_path

  upload_path="gs://jito-$solana_cluster/$epoch/$(hostname)/$file_name"

  echo "$upload_path"
}

get_filepath_in_gcloud() {
  local upload_path=$1

  local file_uploaded

  file_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  echo "$file_uploaded"
}

upload_file_to_gcloud() {
  local filepath=$1
  local gcloud_path=$2

  gcloud storage cp "$filepath" "$gcloud_path"
}

upload_merkle_roots() {
  local merkle_root_path=$1
  local keypair_path=$2
  local rpc_url=$3
  local tip_distribution_program_id=$4

  RUST_LOG=info \
    solana-merkle-root-uploader \
    --merkle-root-path "$merkle_root_path" \
    --keypair-path "$keypair_path" \
    --rpc-url "$rpc_url" \
    --tip-distribution-program-id "$tip_distribution_program_id"
}

main() {
  local epoch_info
  local previous_epoch_final_slot
  local snapshot_file
  local snapshot_gcloud_path
  local maybe_stake_meta
  local stake_meta_gcloud_path
  local stake_meta_filename
  local merkle_tree_filename
  local merkle_tree_filepath
  local maybe_merkle_tree

  check_env_vars_set

  # make sure snapshot location exists and has a genesis file in it
  mkdir -p "$SNAPSHOT_DIR"
  cp "$LEDGER_DIR"/genesis.bin "$SNAPSHOT_DIR"

  # ---------------------------------------------------------------------------
  # Read epoch info off RPC and calculate previous epoch + previous epoch's last slot
  # ---------------------------------------------------------------------------

  epoch_info=$(get_epoch_info "$RPC_URL")
  current_epoch=$(echo "$epoch_info" | jq .result.epoch)
  last_epoch=$((current_epoch - 1))
  current_absolute_slot=$(echo "$epoch_info" | jq .result.absoluteSlot)
  current_slot_index=$(echo "$epoch_info" | jq .result.slotIndex)
  epoch_start_slot=$((current_absolute_slot - current_slot_index))
  previous_epoch_final_slot="$((epoch_start_slot - 1))"

  echo "epoch_info: $epoch_info"
  echo "previous_epoch_final_slot: $previous_epoch_final_slot"

  # ---------------------------------------------------------------------------
  # Take snapshot and upload to gcloud
  # ---------------------------------------------------------------------------

  snapshot_file=$(get_snapshot_filename "$SNAPSHOT_DIR" "$previous_epoch_final_slot")
  if [ -z "$snapshot_file" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "creating snapshot epoch: $last_epoch slot: $previous_epoch_final_slot"

    # note: make sure these filenames match everywhere else in the file
    rm "$SNAPSHOT_DIR/snapshot-*.tar.zst" || true
    rm "$SNAPSHOT_DIR/stake-meta-*.json" || true
    rm "$SNAPSHOT_DIR/merkle-tree-*.json" || true

    snapshot_file=$(create_snapshot_for_slot "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$LEDGER_DIR")
  else
    echo "snapshot file already exists: $SNAPSHOT_DIR/$snapshot_file"
  fi

  snapshot_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$snapshot_file")
  snapshot_in_gcloud=$(get_filepath_in_gcloud "$snapshot_gcloud_path") || true
  if [ -z "$snapshot_in_gcloud" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading snapshot to gcloud epoch: $last_epoch slot: $previous_epoch_final_slot"

    upload_file_to_gcloud "$SNAPSHOT_DIR/$snapshot_file" "$snapshot_gcloud_path"
  else
    echo "snapshot file already uploaded to gcloud at: $snapshot_in_gcloud"
  fi

  # ---------------------------------------------------------------------------
  # Load in snapshot, produce stake metadata, and upload to gcloud
  # ---------------------------------------------------------------------------

  stake_meta_filename=stake-meta-"$previous_epoch_final_slot".json
  stake_meta_filepath="$SNAPSHOT_DIR/$stake_meta_filename"
  maybe_stake_meta=$(ls "$stake_meta_filepath" 2>/dev/null || true)
  if [ -z "$maybe_stake_meta" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "running stake-meta-generator epoch: $last_epoch slot: $previous_epoch_final_slot"
    generate_stake_meta "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$stake_meta_filename" "$TIP_DISTRIBUTION_PROGRAM_ID" "$TIP_PAYMENT_PROGRAM_ID"
  else
    echo "stake-meta already exists: $stake_meta_filepath"
  fi

  stake_meta_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$stake_meta_filename")
  stake_meta_in_gcloud=$(get_filepath_in_gcloud "$stake_meta_gcloud_path") || true
  if [ -z "$stake_meta_in_gcloud" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading stake-meta to gcloud epoch: $last_epoch slot: $previous_epoch_final_slot"
    upload_file_to_gcloud "$stake_meta_filepath" "$stake_meta_gcloud_path"
  else
    echo "stake meta already uploaded to gcloud at: $stake_meta_in_gcloud"
  fi

  # ---------------------------------------------------------------------------
  # Produce merkle tree, upload to gcloud, and upload merkle roots on-chain for
  # the provided keypairs
  # ---------------------------------------------------------------------------

  merkle_tree_filename=merkle-tree-"$previous_epoch_final_slot".json
  merkle_tree_filepath="$SNAPSHOT_DIR/$merkle_tree_filename"
  maybe_merkle_tree=$(ls "$merkle_tree_filepath" 2>/dev/null || true)
  if [ -z "$maybe_merkle_tree" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "running merkle-root-generator epoch: $last_epoch slot: $previous_epoch_final_slot"
    generate_merkle_trees "$stake_meta_filepath" "$merkle_tree_filepath"
  else
    echo "stake-meta already exists at: $merkle_tree_filepath"
  fi

  merkle_tree_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$merkle_tree_filename")
  merkle_tree_in_gcloud=$(get_filepath_in_gcloud "$merkle_tree_gcloud_path") || true
  if [ -z "$merkle_tree_in_gcloud" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading merkle-root to gcloud epoch: $last_epoch slot: $previous_epoch_final_slot"
    upload_file_to_gcloud "$merkle_tree_filepath" "$merkle_tree_gcloud_path"
  else
    echo "merkle tree already uploaded to gcloud at: $merkle_tree_gcloud_path"
  fi

  upload_merkle_roots "$merkle_tree_filepath" "$KEYPAIR" "$RPC_URL" "$TIP_DISTRIBUTION_PROGRAM_ID"

  # ---------------------------------------------------------------------------
  # Claim MEV tips
  # ---------------------------------------------------------------------------

  claim_tips "$merkle_tree_filepath" "$RPC_URL" "$TIP_DISTRIBUTION_PROGRAM_ID" "$KEYPAIR"
}

main
