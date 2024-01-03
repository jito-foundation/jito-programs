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

  echo "Posting slack message: $msg"

  curl -X POST --silent --show-error -d "text=$hostname: $msg" -d "channel=$channel" -H "Authorization: Bearer $bearer" https://slack.com/api/chat.postMessage
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

  if [ -z "$SNAPSHOT_OUTPUT_DIR" ]; then
    echo "SNAPSHOT_OUTPUT_DIR must be set"
    exit 1
  fi

  if [ -z "$SNAPSHOT_ARCHIVE_DIR" ]; then
    echo "SNAPSHOT_ARCHIVE_DIR must be set"
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

  epoch_info=$(curl -X POST --silent --show-error "$rpc_url" -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1, "method":"getEpochInfo"}')
  if [ -z "$epoch_info" ]; then
    echo "ERROR Unable to fetch epoch info."
    exit 1
  fi
  echo "$epoch_info"
}

# returns the snapshot filename, assuming it's present
# return value is just the filename
# cant be known ahead of time because the snapshot filename includes a hash
get_snapshot_filename() {
  local snapshot_dir=$1
  local snapshot_slot=$2

  local snapshot_file

  snapshot_file=$(find "$snapshot_dir" -name "snapshot-${snapshot_slot}-[[:alnum:]]*.tar.zst" -type f -printf "%f\n")
  echo "$snapshot_file"
}

# creates a snapshot for the given slot
create_snapshot_for_slot() {
  local snapshot_slot=$1
  local snapshot_output_dir=$2
  local snapshot_archive_dir=$3
  local ledger_location=$4

  local snapshot_file
  local exit_status

  # produces snapshot in $snapshot_output_dir
  # shellcheck disable=SC2012
  RUST_LOG=info solana-ledger-tool -l "$ledger_location" create-snapshot --snapshot-archive-path "$snapshot_archive_dir" "$snapshot_slot" "$snapshot_output_dir"

  # TODO: figure this out
  # for some reason solana-ledger-tool error doesn't cause this script to exit out, so check status here
  exit_status=$?
  if [[ $exit_status -ne 0 ]]; then
    echo "solana-ledger-tool returned $exit_status, exiting."
    exit $exit_status
  fi
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

generate_stake_meta() {
  local slot=$1
  local snapshot_output_dir=$2
  local stake_meta_filename=$3
  local tip_distribution_program_id=$4
  local tip_payment_program_id=$5

  rm -rf "$snapshot_output_dir"/stake-meta.accounts
  rm -rf "$snapshot_output_dir"/tmp*
  rm -rf /tmp/.tmp* # calculate hash stuff stored here

  RUST_LOG=info solana-stake-meta-generator \
    --ledger-path "$snapshot_output_dir" \
    --tip-distribution-program-id "$tip_distribution_program_id" \
    --out-path "$snapshot_output_dir/$stake_meta_filename" \
    --snapshot-slot "$slot" \
    --tip-payment-program-id "$tip_payment_program_id"

  rm -rf "$snapshot_output_dir"/stake-meta.accounts
  rm -rf "$snapshot_output_dir"/tmp*
  rm -rf /tmp/.tmp* # calculate hash stuff stored here
}

generate_merkle_trees() {
  local stake_meta_filepath=$1
  local merkle_tree_filepath=$2
  local rpc_url=$3

  RUST_LOG=info solana-merkle-root-generator \
    --rpc-url "$rpc_url" \
    --stake-meta-coll-path "$stake_meta_filepath" \
    --out-path "$merkle_tree_filepath"
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

prune_old_snapshots() {
  NUM_SNAPSHOTS_TO_KEEP=3
  local to_delete_stake
  local to_delete_merkle
  local to_delete_snapshot

  # sorts by timestamp in filename
  to_delete_stake=$(find "$SNAPSHOT_OUTPUT_DIR" -type f -name 'stake-meta-[0-9]*.json' | sort | head -n -$NUM_SNAPSHOTS_TO_KEEP)
  to_delete_merkle=$(find "$SNAPSHOT_OUTPUT_DIR" -type f -name 'merkle-tree-[0-9]*.json' | sort | head -n -$NUM_SNAPSHOTS_TO_KEEP)
  to_delete_snapshot=$(find "$SNAPSHOT_OUTPUT_DIR" -type f -name 'snapshot-[0-9]*-[[:alnum:]]*.tar.zst' | sort | head -n -$NUM_SNAPSHOTS_TO_KEEP)

  echo "pruning $(echo "$to_delete_snapshot" | wc -w) snapshots in $SNAPSHOT_OUTPUT_DIR"
  # shellcheck disable=SC2086
  rm -f -v $to_delete_stake
  # shellcheck disable=SC2086
  rm -f -v $to_delete_merkle
  # shellcheck disable=SC2086
  rm -f -v $to_delete_snapshot
}



find_previous_epoch_last_slot() {
  local slot_with_block=$1
  local rpc_url=$2

  block_result=$(curl --silent --show-error "$rpc_url" -X POST -H "Content-Type: application/json" -d "{\"jsonrpc\":\"2.0\",\"id\":1, \"method\":\"getBlock\", \"params\": [$slot_with_block, {\"transactionDetails\": \"none\"}]}" | jq .result)

  while [[ $block_result = null ]]; do
    slot_with_block="$((slot_with_block - 1))"
    block_result=$(curl --silent --show-error "$rpc_url" -X POST -H "Content-Type: application/json" -d "{\"jsonrpc\":\"2.0\",\"id\":1, \"method\":\"getBlock\", \"params\": [$slot_with_block, {\"transactionDetails\": \"none\"}]}" | jq .result)
  done
  echo "$slot_with_block"
}

main() {
  local epoch_info
  local previous_epoch_final_slot
  local snapshot_file
  local snapshot_path
  local snapshot_file_size
  local snapshot_gcloud_path
  local stake_meta_gcloud_path
  local stake_meta_filename
  local merkle_tree_filename
  local merkle_tree_filepath
  local merkle_tree_file_size
  local claimant_amounts
  local num_non_zero_claimants

  check_env_vars_set

  # make sure snapshot location exists and has a genesis file in it
  mkdir -p "$SNAPSHOT_OUTPUT_DIR"
  cp "$LEDGER_DIR"/genesis.bin "$SNAPSHOT_OUTPUT_DIR"

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

  # The last slot in the epoch might not have a block, so search backwards until the last block is found.
  previous_epoch_final_slot=$(find_previous_epoch_last_slot "$previous_epoch_final_slot" "$RPC_URL")

  echo "epoch_info: $epoch_info"
  echo "previous_epoch_final_slot: $previous_epoch_final_slot"

  FILE="$SNAPSHOT_OUTPUT_DIR/$last_epoch.done"
  if [ -f "$FILE" ]; then
    echo "epoch $last_epoch finished uploading, exiting"
    exit 0
  fi

  # ---------------------------------------------------------------------------
  # Take snapshot and upload to gcloud
  # ---------------------------------------------------------------------------

  snapshot_file=$(get_snapshot_filename "$SNAPSHOT_OUTPUT_DIR" "$previous_epoch_final_slot")
  snapshot_path="$SNAPSHOT_OUTPUT_DIR/$snapshot_file"
  if [[ ! -f $snapshot_path ]]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "creating snapshot for epoch: $last_epoch slot: $previous_epoch_final_slot"
    prune_old_snapshots
    create_snapshot_for_slot "$previous_epoch_final_slot" "$SNAPSHOT_OUTPUT_DIR" "$SNAPSHOT_ARCHIVE_DIR" "$LEDGER_DIR"

    # snapshot file should exist now, grab the filename here (snapshot-$slot-$hash.tar.zst)
    snapshot_file=$(get_snapshot_filename "$SNAPSHOT_OUTPUT_DIR" "$previous_epoch_final_slot")
    snapshot_path="$SNAPSHOT_OUTPUT_DIR/$snapshot_file"
  else
    echo "snapshot file already exists at: $snapshot_path"
  fi

  snapshot_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$snapshot_file")
  snapshot_in_gcloud=$(get_filepath_in_gcloud "$snapshot_gcloud_path")
  if [ -z "$snapshot_in_gcloud" ]; then
    snapshot_file_size=$(du -h "$snapshot_path" | awk '{ print $1 }')
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading snapshot ($snapshot_file_size) to gcloud for epoch: $last_epoch slot: $previous_epoch_final_slot. url: https://console.cloud.google.com/storage/browser/jito-mainnet/$last_epoch/$(hostname)"

    upload_file_to_gcloud "$snapshot_path" "$snapshot_gcloud_path"
  else
    echo "snapshot file already uploaded to gcloud at: $snapshot_in_gcloud"
  fi

  # ---------------------------------------------------------------------------
  # Load in snapshot, produce stake metadata, and upload to gcloud
  # ---------------------------------------------------------------------------

  stake_meta_filename=stake-meta-"$previous_epoch_final_slot".json
  stake_meta_filepath="$SNAPSHOT_OUTPUT_DIR/$stake_meta_filename"
  if [[ ! -f $stake_meta_filepath ]]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "running stake-meta-generator for epoch: $last_epoch slot: $previous_epoch_final_slot"
    generate_stake_meta "$previous_epoch_final_slot" "$SNAPSHOT_OUTPUT_DIR" "$stake_meta_filename" "$TIP_DISTRIBUTION_PROGRAM_ID" "$TIP_PAYMENT_PROGRAM_ID"
  else
    echo "stake-meta already exists at: $stake_meta_filepath"
  fi

  stake_meta_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$stake_meta_filename")
  stake_meta_in_gcloud=$(get_filepath_in_gcloud "$stake_meta_gcloud_path")
  if [ -z "$stake_meta_in_gcloud" ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading stake-meta to gcloud for epoch: $last_epoch slot: $previous_epoch_final_slot"
    upload_file_to_gcloud "$stake_meta_filepath" "$stake_meta_gcloud_path"
  else
    echo "stake meta already uploaded to gcloud at: $stake_meta_in_gcloud"
  fi

  # ---------------------------------------------------------------------------
  # Produce merkle tree, upload to gcloud, and upload merkle roots on-chain for
  # the provided keypairs
  # ---------------------------------------------------------------------------

  merkle_tree_filename=merkle-tree-"$previous_epoch_final_slot".json
  merkle_tree_filepath="$SNAPSHOT_OUTPUT_DIR/$merkle_tree_filename"
  if [[ ! -f $merkle_tree_filepath ]]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "running merkle-root-generator for epoch: $last_epoch slot: $previous_epoch_final_slot"
    generate_merkle_trees "$stake_meta_filepath" "$merkle_tree_filepath" "$RPC_URL"
  else
    echo "stake-meta already exists at: $merkle_tree_filepath"
  fi

  merkle_tree_gcloud_path=$(get_gcloud_path "$SOLANA_CLUSTER" "$last_epoch" "$merkle_tree_filename")
  merkle_tree_in_gcloud=$(get_filepath_in_gcloud "$merkle_tree_gcloud_path") || true
  if [ -z "$merkle_tree_in_gcloud" ]; then
    merkle_tree_file_size=$(du -h "$merkle_tree_filepath" | awk '{ print $1 }')
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading merkle-root to gcloud ($merkle_tree_file_size). epoch: $last_epoch slot: $previous_epoch_final_slot"
    upload_file_to_gcloud "$merkle_tree_filepath" "$merkle_tree_gcloud_path"
  else
    echo "merkle tree already uploaded to gcloud at: $merkle_tree_gcloud_path"
  fi

  if [ "${SEND_TRANSACTIONS-false}" = true ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "uploading merkle-root on-chain for epoch: $last_epoch slot: $previous_epoch_final_slot"
    upload_merkle_roots "$merkle_tree_filepath" "$KEYPAIR" "$RPC_URL" "$TIP_DISTRIBUTION_PROGRAM_ID"
  else
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "SEND_TRANSACTIONS flag not set, skipping merkle root upload: $last_epoch slot: $previous_epoch_final_slot"
  fi


  # ---------------------------------------------------------------------------
  # Claim MEV tips
  # ---------------------------------------------------------------------------

  if [ "${SEND_TRANSACTIONS-false}" = true ]; then
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "claiming mev tips for epoch: $last_epoch slot: $previous_epoch_final_slot"
    claim_tips "$merkle_tree_filepath" "$RPC_URL" "$TIP_DISTRIBUTION_PROGRAM_ID" "$KEYPAIR"
  else
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "SEND_TRANSACTIONS flag not set, skipping claim mev tips: $last_epoch slot: $previous_epoch_final_slot"
  fi

  claimant_amounts=$(grep -o -E '"amount": [[:digit:]]+' "$merkle_tree_filepath")
  num_non_zero_claimants=$(echo "$claimant_amounts" | awk '$2 > 0' | wc -l)
  post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "successfully claimed mev tips for epoch: $last_epoch slot: $previous_epoch_final_slot. had $(echo "$claimant_amounts" | wc -l) claimants, $num_non_zero_claimants non-zero lamport claimants."

  # ---------------------------------------------------------------------------
  # Prune old snapshots
  # ---------------------------------------------------------------------------
  prune_old_snapshots

  touch "$SNAPSHOT_OUTPUT_DIR/$last_epoch.done"
}

main
