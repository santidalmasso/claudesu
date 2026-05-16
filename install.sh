#!/bin/sh
# claudesu installer — downloads the prebuilt `csu` binary for this platform.
#
#   curl -fsSL https://github.com/santidalmasso/claudesu/releases/latest/download/install.sh | sh
#
# Environment overrides:
#   CSU_INSTALL_DIR   where to place the binary (default: $HOME/.local/bin)
#   CSU_REPO          GitHub repo to install from (default: santidalmasso/claudesu)

set -eu

REPO="${CSU_REPO:-santidalmasso/claudesu}"
INSTALL_DIR="${CSU_INSTALL_DIR:-$HOME/.local/bin}"

err() { printf 'claudesu: error: %s\n' "$1" >&2; exit 1; }
info() { printf 'claudesu: %s\n' "$1" >&2; }

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-musl" ;;
  *) err "unsupported OS '$os' — on Windows install with: npm install -g claudesu" ;;
esac

case "$arch" in
  x86_64 | amd64)  arch_part="x86_64" ;;
  arm64 | aarch64) arch_part="aarch64" ;;
  *) err "unsupported architecture '$arch'" ;;
esac

asset="csu-${arch_part}-${os_part}"
base="https://github.com/${REPO}/releases/latest/download"

if command -v curl >/dev/null 2>&1; then
  dl() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  dl() { wget -qO "$2" "$1"; }
else
  err "need curl or wget to download the binary"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "downloading ${asset}..."
dl "${base}/${asset}" "${tmp}/csu" || err "download failed — ${base}/${asset}"

# Verify the checksum when sha256 tooling is available (best effort).
if dl "${base}/${asset}.sha256" "${tmp}/csu.sha256" 2>/dev/null; then
  expected="$(awk '{print $1}' "${tmp}/csu.sha256")"
  actual=""
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${tmp}/csu" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "${tmp}/csu" | awk '{print $1}')"
  fi
  if [ -n "$actual" ] && [ "$expected" != "$actual" ]; then
    err "checksum mismatch (expected ${expected}, got ${actual})"
  fi
fi

chmod 755 "${tmp}/csu"
mkdir -p "$INSTALL_DIR"
mv "${tmp}/csu" "${INSTALL_DIR}/csu"
info "installed csu to ${INSTALL_DIR}/csu"

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    info ""
    info "${INSTALL_DIR} is not on your PATH. Add this to your shell profile:"
    info "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac
