#!/usr/bin/env bash
# This script creates a snapshot for the last confirmed slot
# in the previous epoch if one doesn't already exist.
# After creating a snapshot, it creates a snapshot of metadata
# and merkle roots before uploading them on-chain

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck disable=SC1090
source ./"${DIR}"/utils.sh

# make sure all env vars are set for this script
check_env() {
  if [ -z "$RPC_URL" ]; then
    echo "Must pass RPC URL as first arg"
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

  if [ -z "$FEE_PAYER" ]; then
    echo "FEE_PAYER must be set"
    exit 1
  fi

  if [ -z "$KEYPAIR_DIR" ]; then
    echo "KEYPAIR_DIR must be set"
    exit 1
  fi
}

create_snapshot_for_slot() {
  local snapshot_slot=$1
  local snapshot_dir=$2
  local ledger_location=$3

  local snapshot_file
  local snapshot_path

  # shellcheck disable=SC2012
  snapshot_file=$(ls "$snapshot_dir" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })
  if [ -z "$snapshot_file" ]; then
    # if the snapshot we're trying to create doesn't exist, there might be old ones to delete
    clean_old_snapshot_files "$snapshot_dir"
    echo "Didn't find snapshot for slot $snapshot_slot, creating..."
    RUST_LOG=info solana-ledger-tool -l "$ledger_location" create-snapshot "$snapshot_slot"
    # ledger-tool by default updates snapshots in the existing ledger directory
    # and prunes old full/incremental snapshots. copy it out to our snapshot
    # directory when finished creating.
    cp "$ledger_location"*"$snapshot_slot"* "$snapshot_dir"
  else
    echo "Found snapshot, nothing to do."
  fi

  snapshot_path=$(ls "$snapshot_dir" | { grep ".tar.zst" || true; } | { grep "$snapshot_slot" || true; })

  echo "$snapshot_path"
}

upload_snapshot() {
  local epoch_info=$1
  local snapshot_file=$2
  local snapshot_dir=$3
  local solana_cluster=$4

  local current_epoch
  local last_epoch
  local upload_path
  local snapshot_uploaded

  current_epoch=$(echo "$epoch_info" | jq .result.epoch)
  last_epoch=$((current_epoch - 1))

  upload_path="gs://jito-$solana_cluster/$last_epoch/$(hostname)/$snapshot_file"
  snapshot_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$snapshot_uploaded" ]; then
    echo "Snapshot not found in gcp bucket, uploading $snapshot_file to $upload_path"
    gcloud storage cp "$snapshot_dir/$snapshot_file" "$upload_path"
  else
    echo "snapshot already uploaded to gcp at $snapshot_uploaded"
  fi
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

  maybe_snapshot=$(ls "$snapshot_dir" | { grep "$slot" || true; })

  if [ -z "$maybe_snapshot" ]; then
    echo "No snapshot found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    maybe_stake_meta=$(ls "$snapshot_dir"stake-meta-"$slot" 2>/dev/null)
    if [ -z "$maybe_stake_meta" ]; then
      echo "Found snapshot $maybe_snapshot but no stake-meta-$slot, running stake-meta-generator."
      rm -rf "$snapshot_dir"stake-meta.accounts
      rm -rf "$snapshot_dir"tmp*

      RUST_LOG=info solana-stake-meta-generator \
        --ledger-path "$snapshot_dir" \
        --tip-distribution-program-id "$tip_distribution_program_id" \
        --out-path "$snapshot_dir"stake-meta-"$slot" \
        --snapshot-slot "$slot" \
        --tip-payment-program-id "$tip_payment_program_id"

      rm -rf "$snapshot_dir"stake-meta.accounts
      rm -rf "$snapshot_dir"tmp*
    fi
  fi
}

generate_merkle_trees() {
  local slot=$1
  local snapshot_dir=$2
  local rpc_url=$4

  local maybe_stake_meta=$(ls "$snapshot_dir"stake-meta-"$slot" 2>/dev/null)
  if [ -z "$maybe_stake_meta" ]; then
    echo "No stake meta found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    local maybe_merkle_root=$(ls "$snapshot_dir"merkle-root-"$slot"* 2>/dev/null)
    if [ -z "$maybe_merkle_root" ]; then
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
    fi
  fi
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

check_env

epoch_info=$(fetch_epoch_info "$RPC_URL")
previous_epoch_final_slot=$(calculate_previous_epoch_end_slot "$EPOCH_INFO")
echo "previous_epoch_final_slot: $previous_epoch_final_slot"

snapshot_path=$(create_snapshot_for_slot "$previous_epoch_final_slot" "$SNAPSHOT_DIR" "$LEDGER_DIR")
upload_snapshot "$EPOCH_INFO" "$SNAPSHOT_FILE" "$SNAPSHOT_DIR" "$SOLANA_CLUSTER"

generate_stake_meta "$EPOCH_FINAL_SLOT" "$SNAPSHOT_DIR" "$TIP_DISTRIBUTION_PROGRAM_ID" "$TIP_PAYMENT_PROGRAM_ID"
upload_stake_meta "stake-meta" "$EPOCH_INFO" "stake-meta-$EPOCH_FINAL_SLOT"

generate_merkle_trees "$EPOCH_FINAL_SLOT" "$SNAPSHOT_DIR"
upload_merkle_roots "$EPOCH_FINAL_SLOT" "$EPOCH_INFO"

rm_stake_metas "$EPOCH_FINAL_SLOT"
rm_merkle_roots "$EPOCH_FINAL_SLOT"

claim_tips "$EPOCH_FINAL_SLOT"
