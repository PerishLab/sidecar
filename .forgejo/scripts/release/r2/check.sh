#!/usr/bin/env bash
set -euo pipefail

for name in SIDECAR_RELEASES_S3_AK SIDECAR_RELEASES_S3_SK SIDECAR_RELEASES_S3_BUCKET SIDECAR_RELEASES_S3_URL SIDECAR_RELEASES_PUBLIC_URL; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

probe_name=${R2_ACCESS_PROBE_NAME:-release}
probe_key=".probes/$probe_name.txt"
tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT
printf 'sidecar %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$tmpfile"

AWS_ACCESS_KEY_ID="$SIDECAR_RELEASES_S3_AK" \
AWS_SECRET_ACCESS_KEY="$SIDECAR_RELEASES_S3_SK" \
AWS_DEFAULT_REGION=auto \
AWS_EC2_METADATA_DISABLED=true \
aws --endpoint-url "${SIDECAR_RELEASES_S3_URL%/}" s3api put-object \
  --bucket "$SIDECAR_RELEASES_S3_BUCKET" \
  --key "$probe_key" \
  --body "$tmpfile" \
  --content-type "text/plain; charset=utf-8" \
  --no-cli-pager >/dev/null

echo "R2 access ok: $probe_key"
