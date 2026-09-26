#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
  echo "usage: qualify-release-update.sh <bootstrap-archive> <bootstrap-commit> <candidate-commit> <candidate-version>" >&2
  exit 2
fi

bootstrap_archive=$1
bootstrap_commit=$2
candidate_commit=$3
expected_candidate_version=$4

if ! [[ "$bootstrap_commit" =~ ^[0-9a-f]{40}$ ]] ||
   ! [[ "$candidate_commit" =~ ^[0-9a-f]{40}$ ]] ||
   [ "$bootstrap_commit" = "$candidate_commit" ] ||
   ! [[ "$expected_candidate_version" =~ ^[0-9A-Za-z][0-9A-Za-z.+-]{0,63}$ ]]; then
  echo "qualification requires two distinct lowercase 40-hex commits and a valid candidate version" >&2
  exit 2
fi
if [ ! -f "$bootstrap_archive" ] || [ ! -f "$bootstrap_archive.sha256" ]; then
  echo "bootstrap archive or checksum is missing" >&2
  exit 2
fi

archive_dir=$(cd "$(dirname "$bootstrap_archive")" && pwd -P)
archive_name=$(basename "$bootstrap_archive")
(
  cd "$archive_dir"
  shasum -a 256 -c "$archive_name.sha256"
)

qualification_root=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/skm-release-update.XXXXXX")
trap 'rm -rf "$qualification_root"' EXIT
install_dir="$qualification_root/install"
mkdir -p "$install_dir"
tar -C "$install_dir" -xzf "$archive_dir/$archive_name"
skm_path="$install_dir/skm"
chmod 755 "$skm_path"

unset SKM_NO_UPDATE_CHECK
if SKM_NO_UPDATE_CHECK=1 "$skm_path" self version >/dev/null 2>&1; then
  bootstrap_version_args=(self version)
  bootstrap_upgrade_args=(self upgrade)
elif SKM_NO_UPDATE_CHECK=1 "$skm_path" version >/dev/null 2>&1; then
  bootstrap_version_args=(version)
  bootstrap_upgrade_args=(update)
else
  echo "bootstrap binary has no supported version identity command" >&2
  exit 1
fi
bootstrap_version=$(
  SKM_NO_UPDATE_CHECK=1 "$skm_path" "${bootstrap_version_args[@]}"
)
if [[ "$bootstrap_version" =~ ^skm[[:space:]]+([^[:space:]]+)[[:space:]]+\(development[[:space:]]-[[:space:]]$bootstrap_commit\)$ ]]; then
  package_version=${BASH_REMATCH[1]}
else
  echo "unexpected bootstrap identity: $bootstrap_version" >&2
  exit 1
fi
bootstrap_digest=$(shasum -a 256 "$skm_path" | awk '{print $1}')

transcript="$qualification_root/startup-notice.txt"
notice_seen=0
candidate_short=${candidate_commit:0:7}
for attempt in 1 2 3; do
  : >"$transcript"
  if [ "$(uname -s)" = "Darwin" ]; then
    script -q "$transcript" "$skm_path" "${bootstrap_version_args[@]}" >/dev/null 2>&1
  else
    printf -v notice_command '%q ' "$skm_path" "${bootstrap_version_args[@]}"
    script -q -e -c "$notice_command" "$transcript" >/dev/null 2>&1
  fi
  if grep -Fq '[skm] Channel update available:' "$transcript" &&
     grep -Fq "$candidate_short" "$transcript"; then
    notice_seen=1
    break
  fi
done
if [ "$notice_seen" -ne 1 ]; then
  echo "bootstrap binary did not emit the candidate update notice in a terminal" >&2
  sed -n '1,80p' "$transcript" >&2
  exit 1
fi

update_output=$("$skm_path" "${bootstrap_upgrade_args[@]}")
printf '%s\n' "$update_output"
grep -Fq 'Updated skm:' <<<"$update_output"
grep -Fq "${bootstrap_commit:0:7}" <<<"$update_output"
grep -Fq "$candidate_short" <<<"$update_output"

candidate_identity=$(SKM_NO_UPDATE_CHECK=1 "$skm_path" self version)
expected_candidate="skm $expected_candidate_version (development - $candidate_commit)"
if [ "$candidate_identity" != "$expected_candidate" ]; then
  echo "updated executable has the wrong identity: $candidate_identity" >&2
  exit 1
fi
candidate_digest=$(shasum -a 256 "$skm_path" | awk '{print $1}')
if [ "$candidate_digest" = "$bootstrap_digest" ]; then
  echo "update did not replace the bootstrap executable bytes" >&2
  exit 1
fi

noop_output=$("$skm_path" self upgrade)
printf '%s\n' "$noop_output"
grep -Fq 'skm is already up to date:' <<<"$noop_output"
grep -Fq "$candidate_short" <<<"$noop_output"
noop_digest=$(shasum -a 256 "$skm_path" | awk '{print $1}')
if [ "$noop_digest" != "$candidate_digest" ]; then
  echo "already-current update changed the executable" >&2
  exit 1
fi
