#!/usr/bin/env bash
set -euo pipefail

REPO="sylvercode/obsidian-link-resolver"
BINARY_NAME="obsidian-link-resolver"
MODE="install"
VERSION=""
INSTALL_DIR=""

usage() {
	cat <<'EOF'
Usage: install.sh [--mode install|update] [--version <version>] [--install-dir <dir>]

Options:
  --mode         install (default) or update
  --version      Specific version to install, with or without leading 'v'
  --install-dir  Destination directory (defaults based on mode and OS)
  -h, --help     Show this help
EOF
}

fail() {
	echo "error: $*" >&2
	exit 1
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--mode)
		[[ $# -ge 2 ]] || fail "--mode requires a value"
		MODE="$2"
		shift 2
		;;
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

case "$MODE" in
install | update) ;;
*)
	fail "--mode must be 'install' or 'update'"
	;;
esac

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

asset_name="${BINARY_NAME}-${tag_name}-${os_token}-${arch_token}"

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
[[ -n "$download_url" ]] || fail "could not find asset '${asset_name}' in release ${tag_name}"

existing_path=""
if command -v "$BINARY_NAME" >/dev/null 2>&1; then
	existing_path="$(command -v "$BINARY_NAME")"
fi

if [[ -z "$INSTALL_DIR" ]]; then
	if [[ "$MODE" == "update" ]]; then
		[[ -n "$existing_path" ]] || fail "cannot update: '${BINARY_NAME}' is not installed in PATH"
		target_path="$existing_path"
	else
		if [[ "$(id -u)" -eq 0 ]]; then
			INSTALL_DIR="/usr/local/bin"
		else
			INSTALL_DIR="${HOME}/.local/bin"
		fi
		target_path="${INSTALL_DIR}/${BINARY_NAME}"
	fi
else
	target_path="${INSTALL_DIR%/}/${BINARY_NAME}"
fi

target_dir="$(dirname "$target_path")"
mkdir -p "$target_dir"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
tmpfile="${tmpdir}/${BINARY_NAME}"

curl -fsSL --retry 3 --retry-delay 1 "$download_url" -o "$tmpfile" || fail "failed downloading ${asset_name}"
install -m 0755 "$tmpfile" "$target_path" || fail "failed installing to ${target_path}"

echo "Installed ${BINARY_NAME} ${tag_name} to ${target_path}"
echo "Tip: ensure '${target_dir}' is in your PATH"
