#!/usr/bin/env bash
set -euo pipefail

WORKFLOW_FILE="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.github/workflows/ci.yml"
workflow_contents="$(cat "$WORKFLOW_FILE")"

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected to find: $needle" >&2
    exit 1
  fi
}

assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" == *"$needle"* ]]; then
    echo "did not expect to find: $needle" >&2
    exit 1
  fi
}

assert_contains "$workflow_contents" 'sourceparts="$(mktemp -d)"'
assert_contains "$workflow_contents" 'cat <<'\''EOF'\'' > "$sourceparts/ubuntu-archive-amd64.sources"'
assert_contains "$workflow_contents" "URIs: http://azure.archive.ubuntu.com/ubuntu"
assert_contains "$workflow_contents" "Architectures: amd64"
assert_contains "$workflow_contents" 'cat <<'\''EOF'\'' > "$sourceparts/ubuntu-ports-arm64.sources"'
assert_contains "$workflow_contents" "URIs: http://ports.ubuntu.com/ubuntu-ports"
assert_contains "$workflow_contents" "Architectures: arm64"
assert_contains "$workflow_contents" '-o Dir::Etc::sourceparts="$sourceparts"'

assert_not_contains "$workflow_contents" "for src in /etc/apt/sources.list.d/*.sources; do"
assert_not_contains "$workflow_contents" "/etc/apt/arm64-ports.sources.list.d"
assert_not_contains "$workflow_contents" "packages.microsoft.com"

echo "ci arm64 source configuration looks correct"
