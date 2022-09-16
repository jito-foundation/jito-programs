#!/usr/bin/env sh
# This script detects an unprocessed snapshot and
# generates stake meta and merkle roots, before
# claiming all tips on behalf of participants.
# EX: TODO EXAMPLE

RPC_URL=$1
TIP_DISTRIBUTION_PROGRAM_ID=$2
FEE_PAYER=$3
SNAPSHOT_DIR=/home/core/autosnapshot/
KEYPAIR_DIR=/home/core/autosnapshot/keypairs/
STAKE_META_BIN=/home/core/jito-solana/docker-output/solana-stake-meta-generator
MERKLE_ROOT_BIN=/home/core/jito-solana/docker-output/solana-merkle-root-generator
SOLANA_KEYGEN_BIN=/home/core/jito-solana/docker-output/solana-keygen
CLAIM_TIPS_BIN=/home/core/jito-solana/docker-output/claim-mev

check_params () {
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

generate_stake_meta () {
  FOUND_SNAPSHOT=$(ls "$SNAPSHOT_DIR" | grep "$LAST_EPOCH_FINAL_SLOT")

  if [ -z "$FOUND_SNAPSHOT" ]
  then
    echo "No snapshot found for slot $LAST_EPOCH_FINAL_SLOT. Nothing to do. Exiting."
    exit 1
  else
    FOUND_STAKE_META=$(ls "$SNAPSHOT_DIR"stake-meta-"$LAST_EPOCH_FINAL_SLOT" 2> /dev/null)
    if [ -z "$FOUND_STAKE_META" ]
    then
      echo "Found snapshot $FOUND_SNAPSHOT but no stake-meta-$LAST_EPOCH_FINAL_SLOT, running stake-meta-generator."
      RUST_LOG=info "$STAKE_META_BIN" \
        --ledger-path "$SNAPSHOT_DIR" \
        --tip-distribution-program-id "$TIP_DISTRIBUTION_PROGRAM_ID" \
        --out-path "$SNAPSHOT_DIR"stake-meta-"$LAST_EPOCH_FINAL_SLOT" \
        --snapshot-slot "$LAST_EPOCH_FINAL_SLOT" \
        --rpc-url http://"$RPC_URL"
      rm -rf "$SNAPSHOT_DIR"stake-meta.accounts
      rm -rf "$SNAPSHOT_DIR"tmp*
    fi
  fi
}

generate_merkle_trees () {
  FOUND_STAKE_META=$(ls "$SNAPSHOT_DIR"stake-meta-"$LAST_EPOCH_FINAL_SLOT")

  if [ -z "$FOUND_STAKE_META" ]
  then
    echo "No stake meta found for slot $LAST_EPOCH_FINAL_SLOT. Nothing to do. Exiting."
    exit 1
  else
    FOUND_MERKLE_ROOT=$(ls "$SNAPSHOT_DIR"merkle-root-"$LAST_EPOCH_FINAL_SLOT"* 2> /dev/null)
    if [ -z "$FOUND_MERKLE_ROOT" ]
    then
      echo "Found stake-meta-$LAST_EPOCH_FINAL_SLOT but no merkle root, running merkle-root-generator."
      for KEYPAIR_FILE in $(ls "$KEYPAIR_DIR")
      do
        KEYPAIR_PATH="$KEYPAIR_DIR$KEYPAIR_FILE"
        PUBKEY=$("$SOLANA_KEYGEN_BIN" pubkey "$KEYPAIR_PATH")
        echo "Generating merkle root for $PUBKEY"
        RUST_LOG=info "$MERKLE_ROOT_BIN" \
        --path-to-my-keypair "$KEYPAIR_PATH" \
        --rpc-url "http://$RPC_URL" \
        --stake-meta-coll-path "$SNAPSHOT_DIR"stake-meta-"$LAST_EPOCH_FINAL_SLOT" \
        --out-path "$SNAPSHOT_DIR"merkle-root-"$LAST_EPOCH_FINAL_SLOT"-"$PUBKEY" \
        --upload-roots \
        --force-upload-root true
        if [ $? -ne 0 ]
        then
          echo "Detected non-zero exit code. Deleting merkle root."
          rm "$SNAPSHOT_DIR"merkle-root-"$LAST_EPOCH_FINAL_SLOT$PUBKEY"
        else
          echo "Successfully uploaded merkle roots for $PUBKEY"
        fi
      done
    fi
  fi
}

claim_tips () {
    FOUND_MERKLE_ROOT=$(ls "$SNAPSHOT_DIR"merkle-root-"$LAST_EPOCH_FINAL_SLOT"* 2> /dev/null)
    if [ -z "$FOUND_MERKLE_ROOT" ]
    then
      echo "No merkle roots found, unable to claim tips."
      exit 1
    fi
    echo "Found merkle roots for slot $LAST_EPOCH_FINAL_SLOT! Claiming tips."

    for MERKLE_ROOT_FILE in $(ls "$SNAPSHOT_DIR"merkle-root-"$LAST_EPOCH_FINAL_SLOT"*)
    do
      echo "Processing $MERKLE_ROOT_FILE"
      RUST_LOG=info "$CLAIM_TIPS_BIN" \
        --fee-payer "$FEE_PAYER" \
        --merkle-tree "$MERKLE_ROOT_FILE" \
        --tip-distribution-pid "$TIP_DISTRIBUTION_PROGRAM_ID" \
        --url "http://$RPC_URL"
    done
}

check_params
fetch_last_epoch_final_slot
echo "last confirmed slot in previous epoch: $LAST_EPOCH_FINAL_SLOT"

generate_stake_meta
generate_merkle_trees
claim_tips
