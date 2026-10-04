#!/usr/bin/env bash
set -euo pipefail

REPO="sylvercode/obsidian-link-resolver"
BINARY_NAME="obsidian-link-resolver"
VERSION=""
INSTALL_DIR=""

usage() {
	cat <<'EOF'
Usage: install.sh [--version <version>] [--install-dir <dir>]

Behavior:
  Always tries to update the installed binary in PATH, and falls back to a fresh
  install when no existing binary is found.

Options:
  --version      Specific version to install, with or without leading 'v'
  --install-dir  Install destination when no existing binary is found
  -h, --help     Show this help
EOF
}

fail() {
	echo "error: $*" >&2
	exit 1
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--version)
		[[ $# -ge 2 ]] || fail "--version requires a value"
		VERSION="$2"
		shift 2
		;;
	--install-dir)
		[[ $# -ge 2 ]] || fail "--install-dir requires a value"
		INSTALL_DIR="$2"
		shift 2
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		fail "unknown argument: $1"
		;;
	esac
done

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v awk >/dev/null 2>&1 || fail "awk is required"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
Linux) os_token="linux" ;;
Darwin) os_token="darwin" ;;
*)
	fail "unsupported OS: $os (supported: Linux, Darwin)"
	;;
esac

case "$arch" in
x86_64 | amd64) arch_token="x86_64" ;;
aarch64 | arm64) arch_token="aarch64" ;;
*)
	fail "unsupported architecture: $arch (supported: x86_64, aarch64)"
	;;
esac

api_url="https://api.github.com/repos/${REPO}/releases/latest"
if [[ -n "$VERSION" ]]; then
	tag="$VERSION"
	if [[ "$tag" != v* ]]; then
		tag="v${tag}"
	fi
	api_url="https://api.github.com/repos/${REPO}/releases/tags/${tag}"
fi

auth_header=()
if [[ -n "${GH_TOKEN:-}" ]]; then
	auth_header=(-H "Authorization: Bearer ${GH_TOKEN}")
fi

release_json="$(
	curl -fsSL \
		-H "Accept: application/vnd.github+json" \
		"${auth_header[@]}" \
		"$api_url"
)" || fail "failed to fetch release metadata from GitHub API"

tag_name="$(
	printf '%s' "$release_json" | awk -F'"' '/"tag_name":/ { print $4; exit }'
)"
[[ -n "$tag_name" ]] || fail "release metadata missing tag_name"

asset_candidates=()
if [[ "$os_token" == "windows" ]]; then
	asset_candidates+=("${BINARY_NAME}-${tag_name}-${os_token}-${arch_token}.exe" "${BINARY_NAME}-${tag_name}-${os_token}-${arch_token}")
else
	asset_candidates+=("${BINARY_NAME}-${tag_name}-${os_token}-${arch_token}" "${BINARY_NAME}-${tag_name}-${os_token}-${arch_token}.exe")
fi

download_url=""
for asset_name in "${asset_candidates[@]}"; do
	download_url="$(
		printf '%s' "$release_json" | awk -v asset="$asset_name" '
			$0 ~ "\"name\": \"" asset "\"" { found=1; next }
			found && /"browser_download_url":/ {
				gsub(/.*"browser_download_url": "/, "", $0)
				gsub(/",?$/, "", $0)
				print $0
				exit
			}
		'
	)"
	if [[ -n "$download_url" ]]; then
		break
	fi
done
[[ -n "$download_url" ]] || fail "could not find a compatible installer asset for ${os_token}-${arch_token} in release ${tag_name}"

existing_path=""
if command -v "$BINARY_NAME" >/dev/null 2>&1; then
	existing_path="$(command -v "$BINARY_NAME")"
fi

if [[ -n "$INSTALL_DIR" ]]; then
	target_path="${INSTALL_DIR%/}/${BINARY_NAME}"
elif [[ -n "$existing_path" ]]; then
	target_path="$existing_path"
else
	if [[ "$(id -u)" -eq 0 ]]; then
		INSTALL_DIR="/usr/local/bin"
	else
		INSTALL_DIR="${HOME}/.local/bin"
	fi
	target_path="${INSTALL_DIR}/${BINARY_NAME}"
fi

target_dir="$(dirname "$target_path")"
mkdir -p "$target_dir"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
tmpfile="${tmpdir}/${BINARY_NAME}"

curl -fsSL --retry 3 --retry-delay 1 "$download_url" -o "$tmpfile" || fail "failed downloading ${asset_name}"
install -m 0755 "$tmpfile" "$target_path" || fail "failed installing to ${target_path}"

if [[ -n "$existing_path" ]]; then
	echo "Updated ${BINARY_NAME} ${tag_name} at ${target_path}"
else
	echo "Installed ${BINARY_NAME} ${tag_name} to ${target_path}"
	echo "Tip: ensure '${target_dir}' is in your PATH"
fi
