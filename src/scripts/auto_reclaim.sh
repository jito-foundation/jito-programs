#!/usr/bin/env bash
# This script reclaims rent from claim status accounts and tip distribution accounts.
# NOTE: this file depends on binaries being built in jito-solana

# error out, unset variables are errors, and echo commands
set -eux

RPC_URL=$1
TIP_DISTRIBUTION_PROGRAM_ID=$2
KEYPAIR_PATH=$3
SLACK_APP_TOKEN=$4
SLACK_CHANNEL=$5

# make sure all env vars are set for this script
check_env_vars_set() {
  if [ -z "$RPC_URL" ]; then
    echo "RPC_URL must be set"
    exit 1
  fi

  if [ -z "$TIP_DISTRIBUTION_PROGRAM_ID" ]; then
    echo "TIP_DISTRIBUTION_PROGRAM_ID must be set"
    exit 1
  fi

  if [ -z "$KEYPAIR_PATH" ]; then
    echo "KEYPAIR_PATH must be set"
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

post_slack_message() {
  local bearer=$1
  local channel=$2
  local msg=$3

  local hostname

  hostname=$(hostname)

  echo "Posting slack message: $msg"

  curl -X POST --silent --show-error -d "text=$hostname: $msg" -d "channel=$channel" -H "Authorization: Bearer $bearer" https://slack.com/api/chat.postMessage
}

main() {
  check_env_vars_set

  post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "reclaiming rent"
  RUST_LOG=info \
      solana-reclaim-rent \
      --rpc-url "$RPC_URL" \
      --keypair-path "$KEYPAIR_PATH" \
      --tip-distribution-program-id "$TIP_DISTRIBUTION_PROGRAM_ID" \
      --should-reclaim-tdas
  post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "done reclaiming rent"
}

main "$@"