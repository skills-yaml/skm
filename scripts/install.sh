#!/usr/bin/env sh
set -eu

repo="${SKM_REPO:-skills-yaml/skm}"
channel="${1:-${SKM_CHANNEL:-prod}}"
install_dir="${SKM_INSTALL_DIR:-$HOME/.local/bin}"

case "$channel" in
  prod|production)
    tag="prod-latest"
    ;;
  dev|development)
    tag="development-latest"
    ;;
  *)
    echo "Unsupported channel: $channel" >&2
    echo "Use: prod or development" >&2
    exit 1
    ;;
esac
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)
    case "$arch" in
      x86_64|amd64)
        asset="skm-linux-x86_64.tar.gz"
        ;;
      *)
        echo "Unsupported Linux architecture: $arch" >&2
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$arch" in
      x86_64|amd64)
        asset="skm-macos-x86_64.tar.gz"
        ;;
      arm64|aarch64)
        asset="skm-macos-aarch64.tar.gz"
        ;;
      *)
        echo "Unsupported macOS architecture: $arch" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $os" >&2
    exit 1
    ;;
esac

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

url="https://github.com/$repo/releases/download/$tag/$asset"
checksum_url="$url.sha256"
mkdir -p "$install_dir"

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp_dir/$asset"
curl -fsSL "$checksum_url" -o "$tmp_dir/$asset.sha256"

expected_hash="$(awk 'NR == 1 { print tolower($1) }' "$tmp_dir/$asset.sha256")"
case "$expected_hash" in
  ''|*[!0-9a-f]*)
    echo "Invalid checksum file: $checksum_url" >&2
    exit 1
    ;;
esac
if [ "${#expected_hash}" -ne 64 ]; then
  echo "Invalid SHA-256 digest in: $checksum_url" >&2
  exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  actual_hash="$(sha256sum "$tmp_dir/$asset" | awk '{ print tolower($1) }')"
elif command -v shasum >/dev/null 2>&1; then
  actual_hash="$(shasum -a 256 "$tmp_dir/$asset" | awk '{ print tolower($1) }')"
else
  echo "SHA-256 verification requires sha256sum or shasum" >&2
  exit 1
fi
if [ "$actual_hash" != "$expected_hash" ]; then
  echo "Checksum verification failed for $asset" >&2
  exit 1
fi
echo "Verified SHA-256 for $asset"

tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
install -m 755 "$tmp_dir/skm" "$install_dir/skm"

echo "Installed skm to $install_dir/skm"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    echo "Add this directory to PATH if needed:"
    echo "  $install_dir"
    ;;
esac
