#!/usr/bin/env sh
# This script detects an unprocessed snapshot and
# generates stake meta and merkle roots, before
# claiming all tips on behalf of participants.

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
source ./${DIR}/utils.sh

RPC_URL=$1
HOST_NAME=$2

TIP_DISTRIBUTION_PROGRAM_ID=$TIP_DISTRIBUTION_PROGRAM_ID
TIP_PAYMENT_PROGRAM_ID=$TIP_PAYMENT_PROGRAM_ID
FEE_PAYER=$FEE_PAYER
SNAPSHOT_DIR=$SNAPSHOT_DIR
KEYPAIR_DIR=$KEYPAIR_DIR
SOLANA_CLUSTER=$SOLANA_CLUSTER

check_env() {
  if [ -z "$RPC_URL" ]; then
    echo "Must pass RPC URL as first arg"
    exit 1
  fi

  if [ -z "$HOST_NAME" ]; then
    echo "Must pass host name as second arg"
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

  if [ -z "$SNAPSHOT_DIR" ]; then
    echo "SNAPSHOT_DIR must be set"
    exit 1
  fi

  if [ -z "$KEYPAIR_DIR" ]; then
    echo "KEYPAIR_DIR must be set"
    exit 1
  fi

  if [ -z "$SOLANA_CLUSTER" ]; then
    echo "SOLANA_CLUSTER must be set"
    exit 1
  fi
}

generate_stake_meta() {
  local slot=$1

  # shellcheck disable=SC2012
  local maybe_snapshot=$(ls "$SNAPSHOT_DIR" | { grep "$slot" || true; })

  if [ -z "$maybe_snapshot" ]; then
    echo "No snapshot found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    local maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$slot" 2>/dev/null)
    if [ -z "$maybe_stake_meta" ]; then
      echo "Found snapshot $maybe_snapshot but no stake-meta-$slot, running stake-meta-generator."
      RUST_LOG=info solana-stake-meta-generator \
        --ledger-path "$SNAPSHOT_DIR" \
        --tip-distribution-program-id "$TIP_DISTRIBUTION_PROGRAM_ID" \
        --out-path "$SNAPSHOT_DIR"stake-meta-"$slot" \
        --snapshot-slot "$slot" \
        --tip-payment-program-id "$TIP_PAYMENT_PROGRAM_ID"
      rm -rf "$SNAPSHOT_DIR"stake-meta.accounts
      rm -rf "$SNAPSHOT_DIR"tmp*
    fi
  fi
}

generate_merkle_trees() {
  local slot=$1

  local maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$slot" 2>/dev/null)
  if [ -z "$maybe_stake_meta" ]; then
    echo "No stake meta found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    local maybe_merkle_root=$(ls "$SNAPSHOT_DIR"merkle-root-"$slot"* 2>/dev/null)
    if [ -z "$maybe_merkle_root" ]; then
      echo "Found stake-meta-$slot but no merkle root, running merkle-root-generator."
      # shellcheck disable=SC2045
      for keypair_file in $(ls "$KEYPAIR_DIR"); do
        local keypair_path="$KEYPAIR_DIR$keypair_file"
        echo "keypair_path: $keypair_path"

        pubkey=$(solana-keygen pubkey "$keypair_path")
        echo "Generating merkle root for $pubkey"

        RUST_LOG=info solana-merkle-root-generator \
          --path-to-my-keypair "$keypair_path" \
          --rpc-url "http://$RPC_URL" \
          --stake-meta-coll-path "$SNAPSHOT_DIR"stake-meta-"$slot" \
          --out-path "$SNAPSHOT_DIR"merkle-root-"$slot"-"$pubkey" \
          --upload-roots \
          --force-upload-root true
        if [ $? -ne 0 ]; then
          echo "Detected non-zero exit code. Deleting merkle root."
          rm "$SNAPSHOT_DIR"merkle-root-"$slot$pubkey"
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

upload_file() {
  local name=$1
  local epoch_info=$2
  local file_name=$3

  local epoch=$(echo "$epoch_info" | jq .result.epoch)
  local prev_epoch=$((epoch - 1))
  local upload_path="gs://jito-$SOLANA_CLUSTER/$prev_epoch/$HOST_NAME/$file_name"
  local file_uploaded=$(gcloud storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$file_uploaded" ]; then
    echo "$name not found in gcp bucket, uploading now."
    echo "upload_path: $upload_path"
    echo "file_name: $file_name"
    gcloud storage cp "$SNAPSHOT_DIR""$file_name" "$upload_path"
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
    upload_file "merkle-root for $pubkey" "$epoch_info" "merkle-root-$slot-$pubkey"
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

fetch_epoch_info "$RPC_URL"
calculate_epoch_end_slot "$EPOCH_INFO"
echo "last confirmed slot in previous epoch: $EPOCH_FINAL_SLOT"

generate_stake_meta "$EPOCH_FINAL_SLOT"

upload_file "stake-meta" "$EPOCH_INFO" "stake-meta-$EPOCH_FINAL_SLOT"
generate_merkle_trees "$EPOCH_FINAL_SLOT"

upload_merkle_roots "$EPOCH_FINAL_SLOT" "$EPOCH_INFO"

rm_stake_metas "$EPOCH_FINAL_SLOT"
rm_merkle_roots "$EPOCH_FINAL_SLOT"

claim_tips "$EPOCH_FINAL_SLOT"
