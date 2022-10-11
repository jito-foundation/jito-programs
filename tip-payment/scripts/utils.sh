#!/usr/bin/env sh
set -e

fetch_epoch_info() {
  local rpc_url=$1

  EPOCH_INFO=$(curl -s "http://$rpc_url" -X POST -H "Content-Type: application/json" -d '
      {"jsonrpc":"2.0","id":1, "method":"getEpochInfo"}
    ')
  if [ -z "$EPOCH_INFO" ]
  then
    echo "ERROR Unable to fetch epoch info."
    exit 1
  fi
}

calculate_epoch_end_slot() {
  local epoch_info=$1

  local current_absolute_slot=$(echo "$epoch_info" | jq .result.absoluteSlot)
  local current_slot_index=$(echo "$epoch_info" | jq .result.slotIndex)
  local epoch_start_slot=$((current_absolute_slot - current_slot_index))

  EPOCH_FINAL_SLOT=$((epoch_start_slot - 1))
}

fetch_highest_confirmed_slot() {
  local epoch_end_slot=$1

  # Due to possible forking / disconnected blocks at the end of the last epoch
  # we check within a 40 slot range for the highest confirmed block
  local range_begin=$((last_epoch_end_slot - 40))

  HIGHEST_CONFIRMED_SLOT=$(curl -s "http://$rpc_url" -X POST -H "Content-Type: application/json" -d "
    {\"jsonrpc\": \"2.0\",\"id\":1,\"method\":\"getBlocks\",\"params\":[$range_begin, $last_epoch_end_slot]}
  " | jq '.result | last')

  if [[ "$HIGHEST_CONFIRMED_SLOT" == "null" ]]
  then
    echo "Missing block range [$range_begin, $HIGHEST_CONFIRMED_SLOT] for last epoch. Nothing to do. Exiting."
    exit 1
  fi
}