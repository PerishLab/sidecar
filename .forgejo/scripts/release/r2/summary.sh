#!/usr/bin/env bash
set -euo pipefail

{
  echo "## sidecar release"
  echo
  echo "- metadata: ${R2_METADATA_URL:-}"
  echo "- version metadata: ${R2_VERSION_METADATA_URL:-}"
  echo "- version prefix: ${R2_VERSION_PREFIX:-}"
} >> "$GITHUB_STEP_SUMMARY"

