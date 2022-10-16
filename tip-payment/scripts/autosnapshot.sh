#!/usr/bin/env bash
# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.
# After creating a snapshot, it creates a snapshot of metadata
# and merkle roots before uploading them on-chain

# error out, unset variables are errors, and echo commands
set -eux

RPC_URL=$1

# make sure all env vars are set for this script
check_env_vars_set() {
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

  if [ -z "$FEE_PAYER" ]; then
    echo "FEE_PAYER must be set"
    exit 1
  fi

  if [ -z "$KEYPAIR_DIR" ]; then
    echo "KEYPAIR_DIR must be set"
    exit 1
  fi

  if [ -z "$LEDGER_DIR" ]; then
    echo "LEDGER_DIR must be set"
    exit 1
  fi
}

# read epoch info from RPC endpoint
get_epoch_info() {
  local rpc_url=$1

  local epoch_info

  epoch_info=$(curl "http://$rpc_url" -X POST -H "Content-Type: application/json" -d '
      {"jsonrpc":"2.0","id":1, "method":"getEpochInfo"}
    ')
  if [ -z "$epoch_info" ]; then
    echo "ERROR Unable to fetch epoch info."
    exit 1
  fi
  echo "$epoch_info"
}

# returns the previous epoch's last slot
calculate_previous_epoch_last_slot() {
  local epoch_info=$1

  local current_slot_index
  local current_absolute_slot
  local epoch_start_slot

  current_absolute_slot=$(echo "$epoch_info" | jq .result.absoluteSlot)
  current_slot_index=$(echo "$epoch_info" | jq .result.slotIndex)
  epoch_start_slot=$((current_absolute_slot - current_slot_index))

  echo "$((epoch_start_slot - 1))"
}

fetch_highest_confirmed_slot() {
  local epoch_end_slot=$1

  # Due to possible forking / disconnected blocks at the end of the last epoch
  # we check within a 40 slot range for the highest confirmed block
  local range_begin=$((last_epoch_end_slot - 40))

  HIGHEST_CONFIRMED_SLOT=$(curl "http://$rpc_url" -X POST -H "Content-Type: application/json" -d "
    {\"jsonrpc\": \"2.0\",\"id\":1,\"method\":\"getBlocks\",\"params\":[$range_begin, $last_epoch_end_slot]}
  " | jq '.result | last')

  if [[ "$HIGHEST_CONFIRMED_SLOT" == "null" ]]; then
    echo "Missing block range [$range_begin, $HIGHEST_CONFIRMED_SLOT] for last epoch."
    exit 1
  fi
}

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

  # shellcheck disable=SC2012
  RUST_LOG=info solana-ledger-tool -l "$ledger_location" create-snapshot "$snapshot_slot"
  # ledger-tool by default updates snapshots in the existing ledger directory
  # and prunes old full/incremental snapshots. copy it out to our snapshot
  # directory when finished creating.
  cp "$ledger_location"*"$snapshot_slot"* "$snapshot_dir"

  # snapshot file should exist now, grab the filename here (snapshot-$slot-$hash.tar.zst)
  snapshot_file=$(get_snapshot_filename "$snapshot_dir" "$snapshot_slot")

  echo "$snapshot_file"
}

get_snapshot_gcloud_file() {
  local upload_path=$1

  snapshot_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })
  echo "$snapshot_uploaded"
}

get_gcloud_upload_path() {
  local solana_cluster=$1
  local last_epoch=$2
  local snapshot_file=$3

  upload_path="gs://jito-$solana_cluster/$last_epoch/$(hostname)/$snapshot_file"

  echo "$upload_path"
}

upload_snapshot() {
  local last_epoch=$1
  local snapshot_dir=$2
  local solana_cluster=$3
  local snapshot_file=$4

  local upload_path
  local snapshot_uploaded

  upload_path=$(get_gcloud_upload_path "$solana_cluster" "$last_epoch" "$snapshot_file")
  gcloud storage cp "$snapshot_dir/$snapshot_file" "$upload_path"
}

clean_old_snapshot_files() {
  local snapshot_dir=$1

  # shellcheck disable=SC2012
  maybe_snapshot=$(ls "$snapshot_dir"snapshot* 2>/dev/null | { grep -E "snapshot" || true; })
  if [ -z "$maybe_snapshot" ]; then
    echo "No snapshots to clean up."
  else
    rm "$maybe_snapshot"
  fi
}

generate_stake_meta() {
  local slot=$1
  local snapshot_dir=$2
  local tip_distribution_program_id=$3
  local tip_payment_program_id=$4

  local maybe_snapshot
  local maybe_stake_meta

  rm -rf "$snapshot_dir"stake-meta.accounts
  rm -rf "$snapshot_dir"tmp*

  RUST_LOG=info solana-stake-meta-generator \
    --ledger-path "$snapshot_dir" \
    --tip-distribution-program-id "$tip_distribution_program_id" \
    --out-path "$snapshot_dir"stake-meta-"$slot".json \
    --snapshot-slot "$slot" \
    --tip-payment-program-id "$tip_payment_program_id"

  rm -rf "$snapshot_dir"stake-meta.accounts
  rm -rf "$snapshot_dir"tmp*
}

generate_merkle_trees() {
  local slot=$1
  local snapshot_dir=$2
  local rpc_url=$4
    echo "Found stake-meta-$slot but no merkle root, running merkle-root-generator."
          # shellcheck disable=SC2045
          for keypair_file in $(ls "$keypair_dir"); do
            local keypair_path="$keypair_dir$keypair_file"
            echo "keypair_path: $keypair_path"

            pubkey=$(solana-keygen pubkey "$keypair_path")
            echo "Generating merkle root for $pubkey"

            RUST_LOG=info solana-merkle-root-generator \
              --path-to-my-keypair "$keypair_path" \
              --rpc-url "http://$rpc_url" \
              --stake-meta-coll-path "$snapshot_dir"stake-meta-"$slot" \
              --out-path "$snapshot_dir"merkle-root-"$slot"-"$pubkey" \
              --upload-roots \
              --force-upload-root true
            if [ $? -ne 0 ]; then
              echo "Detected non-zero exit code. Deleting merkle root."
              rm "$snapshot_dir"merkle-root-"$slot$pubkey"
            else
              echo "Successfully uploaded merkle roots for $pubkey"
            fi
          done
}

claim_tips() {
  local slot=$1

  local maybe_merkle_roots=$(ls "$SNAPSHOT_DIR"merkle-root-"$slot"* 2>/dev/null)
  if [ -z "$maybe_merkle_roots" ]; then
    echo "No merkle roots found, unable to claim tips."
    exit 1
  fi
  echo "Found merkle roots for slot $slot! Claiming tips."

  # shellcheck disable=SC2045
  for merkle_root in $(ls "$SNAPSHOT_DIR"merkle-root-"$slot"*); do
    echo "Processing $merkle_root"
    RUST_LOG=info claim-mev \
      --fee-payer "$FEE_PAYER" \
      --merkle-tree "$merkle_root" \
      --tip-distribution-pid "$TIP_DISTRIBUTION_PROGRAM_ID" \
      --url "http://$RPC_URL"
  done
}

upload_stake_meta() {
  local name=$1
  local epoch_info=$2
  local file_name=$3
  local solana_cluster=$4
  local snapshot_dir=$5

  local epoch
  local prev_epoch
  local upload_path
  local file_uploaded

  epoch=$(echo "$epoch_info" | jq .result.epoch)
  prev_epoch=$((epoch - 1))
  upload_path="gs://jito-$solana_cluster/$prev_epoch/$(hostname)/$file_name"
  file_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$file_uploaded" ]; then
    echo "stake meta $name not found in gcp bucket, uploading now."
    echo "upload_path: $upload_path"
    echo "file_name: $file_name"
    gcloud storage cp "$snapshot_dir""$file_name" "$upload_path"
  else
    echo "$name already uploaded to gcp."
  fi
}

upload_merkle_roots() {
  local slot=$1
  local epoch_info=$2

  # shellcheck disable=SC2045
  for keypair_file in $(ls "$KEYPAIR_DIR"); do
    local keypair_path="$KEYPAIR_DIR$keypair_file"
    local pubkey=$(solana-keygen pubkey "$keypair_path")
    upload_stake_meta "merkle-root for $pubkey" "$epoch_info" "merkle-root-$slot-$pubkey"
  done
}

## TODO: loop over
rm_stake_metas() {
  local slot=$1

  # shellcheck disable=SC2012
  ls "$SNAPSHOT_DIR"stake-meta* | { grep -e "$slot" || true; } | xargs rm
}

rm_merkle_roots() {
  local slot=$1

  # shellcheck disable=SC2012
  ls "$SNAPSHOT_DIR"merkle-root* | { grep -e "$slot" || true; } | xargs rm
}

main() {
  local epoch_info
  local previous_epoch_final_slot
  local snapshot_file
  local gcloud_upload_path
  local maybe_stake_meta
  local maybe_merkle_root

  check_env_vars_set

  epoch_info=$(get_epoch_info "$RPC_URL")
  current_epoch=$(echo "$epoch_info" | jq .result.epoch)
  last_epoch=$((current_epoch - 1))
  current_absolute_slot=$(echo "$epoch_info" | jq .result.absoluteSlot)
  current_slot_index=$(echo "$epoch_info" | jq .result.slotIndex)
  epoch_start_slot=$((current_absolute_slot - current_slot_index))
  previous_epoch_final_slot="$((epoch_start_slot - 1))"

  echo "epoch_info: $epoch_info"
  echo "previous_epoch_final_slot: $previous_epoch_final_slot"

  snapshot_file=$(get_snapshot_filename "$SNAPSHOT_DIR" "$previous_epoch_final_slot")
  if [ -z "$snapshot_file" ]; then
    echo "creating snapshot at slot $snapshot_slot"
    snapshot_file=$(create_snapshot_for_slot "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$LEDGER_DIR")
  else
    echo "snapshot file already exists: $snapshot_file"
  fi

  gcloud_upload_path=$(get_gcloud_upload_path "$SOLANA_CLUSTER" "$last_epoch" "$snapshot_file")
  if [ -z "$gcloud_upload_path" ]; then
    echo "uploading $snapshot_dir/$snapshot_file to gcloud path $upload_path"
    upload_snapshot "$last_epoch" "$SNAPSHOT_DIR" "$SOLANA_CLUSTER" "$snapshot_file"
  else
    echo "snapshot file already uploaded to gcloud at: $gcloud_upload_path"
  fi

  maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$previous_epoch_final_slot".json 2>/dev/null || true)
  if [ -z "$maybe_stake_meta" ]; then
    echo "Running stake-meta-generator for slot $$previous_epoch_final_slot"
    generate_stake_meta "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$TIP_DISTRIBUTION_PROGRAM_ID" "$TIP_PAYMENT_PROGRAM_ID"
    maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$previous_epoch_final_slot".json 2>/dev/null)
  else
    echo "stake-meta already exists at: $maybe_stake_meta"
  fi

  maybe_merkle_root=$(ls "$snapshot_dir"merkle-root-"$slot".json 2>/dev/null || true)
  if [ -z "$maybe_merkle_root" ]; then
    echo "Running stake-meta-generator for slot $$previous_epoch_final_slot"
    generate_stake_meta "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$TIP_DISTRIBUTION_PROGRAM_ID" "$TIP_PAYMENT_PROGRAM_ID"
    maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$previous_epoch_final_slot".json 2>/dev/null)
  else
    echo "stake-meta already exists at: $maybe_stake_meta"
  fi
  #  generate_merkle_trees "$EPOCH_FINAL_SLOT" "$SNAPSHOT_DIR"
  #
  #  upload_snapshot "$epoch_info" "$SNAPSHOT_DIR" "$SOLANA_CLUSTER" "$snapshot_file"
  #  upload_stake_meta "stake-meta" "$epoch_info" "stake-meta-$EPOCH_FINAL_SLOT"
  #  upload_merkle_roots "$EPOCH_FINAL_SLOT" "$epoch_info"
  #
  #  rm_stake_metas "$EPOCH_FINAL_SLOT"
  #  rm_merkle_roots "$EPOCH_FINAL_SLOT"
  #
  #  claim_tips "$EPOCH_FINAL_SLOT"
}

main
