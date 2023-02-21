#!/usr/bin/env bash
# This script is an error handling wrapper for autosnapshot_inner.sh

set -euo pipefail
DIR=$(realpath "$(dirname "${BASH_SOURCE[0]}")")

# copy pasted from autosnapshot_inner.sh
post_slack_message() {
  local bearer=$1
  local channel=$2
  local msg=$3
  local hostname

  hostname=$(hostname)

  echo "Posting slack message: $msg"

  curl -d "text=$hostname: $msg" -d "channel=$channel" -H "Authorization: Bearer $bearer" -X POST https://slack.com/api/chat.postMessage
}

main() {
  if ! "$DIR"/autosnapshot_inner.sh "$@"; then
    NUM_LOG_LINES=10
    LOG_SNIPPET=$(journalctl -u autosnapshot --pager-end --lines $NUM_LOG_LINES)
    post_slack_message "$SLACK_APP_TOKEN" "$SLACK_CHANNEL" "autosnapshot failed. journalctl log snippet:
  \`\`\`$LOG_SNIPPET}\`\`\`"
    exit 0
  fi
}

main "$@"
