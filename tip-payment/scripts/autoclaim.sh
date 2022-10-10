#!/usr/bin/env sh

# This script detects an unprocessed snapshot and
# generates stake meta and merkle roots, before
# claiming all tips on behalf of participants.
# EX: TODO EXAMPLE

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
source ./${DIR}/utils.sh

RPC_URL=$1
TIP_DISTRIBUTION_PROGRAM_ID=$2
FEE_PAYER=$3
SNAPSHOT_DIR=/home/core/autosnapshot/
KEYPAIR_DIR=/home/core/autosnapshot/keypairs/
STAKE_META_BIN=/home/core/jito-solana/docker-output/solana-stake-meta-generator
MERKLE_ROOT_BIN=/home/core/jito-solana/docker-output/solana-merkle-root-generator
SOLANA_KEYGEN_BIN=/home/core/jito-solana/docker-output/solana-keygen
CLAIM_TIPS_BIN=/home/core/jito-solana/docker-output/claim-mev
GCLOUD_PATH=/home/core/google-cloud-sdk/bin/gcloud

check_params() {
  if [ -z "$RPC_URL" ]
  then
    echo "Please pass rpc url as first parameter to autoclaim"
    exit
  fi

  if [ -z "$TIP_DISTRIBUTION_PROGRAM_ID" ]
  then
    echo "Please pass tip distribution program id as second parameter to autoclaim"
    exit
  fi

  if [ -z "$FEE_PAYER" ]
  then
    echo "Please pass fee payer keypair file as third parameter to autoclaim"
    exit
  fi
}

generate_stake_meta() {
  local slot=$1

  # shellcheck disable=SC2012
  local maybe_snapshot=$(ls "$SNAPSHOT_DIR" | { grep "$slot" || true; })

  if [ -z "$maybe_snapshot" ]
  then
    echo "No snapshot found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    local maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$slot" 2> /dev/null)
    if [ -z "$maybe_stake_meta" ]
    then
      echo "Found snapshot $maybe_snapshot but no stake-meta-$slot, running stake-meta-generator."
      RUST_LOG=info "$STAKE_META_BIN" \
        --ledger-path "$SNAPSHOT_DIR" \
        --tip-distribution-program-id "$TIP_DISTRIBUTION_PROGRAM_ID" \
        --out-path "$SNAPSHOT_DIR"stake-meta-"$LAST_EPOCH_FINAL_SLOT" \
        --snapshot-slot "$slot" \
        --rpc-url http://"$RPC_URL"
      rm -rf "$SNAPSHOT_DIR"stake-meta.accounts
      rm -rf "$SNAPSHOT_DIR"tmp*
    fi
  fi
}

generate_merkle_trees() {
  local slot=$1

  local maybe_stake_meta=$(ls "$SNAPSHOT_DIR"stake-meta-"$slot")
  if [ -z "$maybe_stake_meta" ]
  then
    echo "No stake meta found for slot $slot. Nothing to do. Exiting."
    exit 1
  else
    local maybe_merkle_root=$(ls "$SNAPSHOT_DIR"merkle-root-"$slot"* 2> /dev/null)
    if [ -z "$maybe_merkle_root" ]
    then
      echo "Found stake-meta-$slot but no merkle root, running merkle-root-generator."
      # shellcheck disable=SC2045
      for keypair_file in $(ls "$KEYPAIR_DIR")
      do
        local keypair_path="$KEYPAIR_DIR$keypair_file"
        local pubkey =$("$SOLANA_KEYGEN_BIN" pubkey "$KEYPAIR_PATH")
        echo "Generating merkle root for $pubkey"

        RUST_LOG=info "$MERKLE_ROOT_BIN" \
        --path-to-my-keypair "$KEYPAIR_PATH" \
        --rpc-url "http://$RPC_URL" \
        --stake-meta-coll-path "$SNAPSHOT_DIR"stake-meta-"$slot" \
        --out-path "$SNAPSHOT_DIR"merkle-root-"$slot"-"$pubkey" \
        --upload-roots \
        --force-upload-root true
        if [ $? -ne 0 ]
        then
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

  local maybe_merkle_roots=$(ls "$SNAPSHOT_DIR"merkle-root-"$slot"* 2> /dev/null)
  if [ -z "$maybe_merkle_roots" ]
  then
    echo "No merkle roots found, unable to claim tips."
    exit 1
  fi
  echo "Found merkle roots for slot $slot! Claiming tips."

  # shellcheck disable=SC2045
  for merkle_root in $(ls "$SNAPSHOT_DIR"merkle-root-"$slot"*)
  do
    echo "Processing $merkle_root"
    RUST_LOG=info "$CLAIM_TIPS_BIN" \
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
  local upload_path="gs://jito-mainnet/$prev_epoch/$file_name"
  local file_uploaded=$($GCLOUD_PATH storage ls "$upload_path" | { grep "$upload_path" || true; })

  if [ -z "$file_uploaded" ]
  then
    echo "$name not found in gcp bucket, uploading now."
    echo "upload_path: $upload_path"
    echo "upload_path: $file_name"
    $GCLOUD_PATH storage cp $SNAPSHOT_DIR/"$file_name" "$upload_path"
  else
    echo "$name already uploaded to gcp."
  fi
}

upload_merkle_roots() {
  local slot=$1
  local epoch_info=$2

  # shellcheck disable=SC2045
  for keypair_file in $(ls "$KEYPAIR_DIR")
  do
    local keypair_path="$KEYPAIR_DIR$keypair_file"
    local pubkey=$("$SOLANA_KEYGEN_BIN" pubkey "$keypair_path")
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
  ls "$SNAPSHOT_DIR"merkle-root* | { grep -v "$slot" || true; } | xargs rm
}

check_params

epoch_info=$(fetch_epoch_info "$RPC_URL" | tail -n 1)
epoch_final_slot=$(calculate_epoch_end_slot "$epoch_info" | tail -n 1)
echo "last confirmed slot in previous epoch: $epoch_final_slot"

generate_stake_meta "$epoch_final_slot"

upload_file "stake-meta" "$epoch_info" "stake-meta-$epoch_final_slot"
generate_merkle_trees "$epoch_final_slot"

upload_merkle_roots "$epoch_final_slot" "$epoch_info"

rm_stake_metas "$epoch_final_slot"
rm_merkle_roots "$epoch_final_slot"

claim_tips "$epoch_final_slot"
