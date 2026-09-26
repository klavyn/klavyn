#!/bin/sh
# klavyn installer — https://klavyn.app
#
#   curl -fsSL https://klavyn.app/install.sh | sh
#
# Tries a prebuilt release binary for your platform; if none matches,
# falls back to building from source with cargo. macOS & Linux.
#
# Env overrides:
#   KLAVYN_BIN_DIR   install destination (default: ~/.local/bin)
#   KLAVYN_REPO      source repo        (default: nickvigilante/klavyn)

set -eu

REPO="${KLAVYN_REPO:-nickvigilante/klavyn}"
BIN_DIR="${KLAVYN_BIN_DIR:-$HOME/.local/bin}"
BIN_NAME="klavyn"

# --- pretty output ---------------------------------------------------------
if [ -t 1 ]; then
  C_CYAN="$(printf '\033[36m')"; C_VIO="$(printf '\033[35m')"
  C_DIM="$(printf '\033[2m')";  C_RED="$(printf '\033[31m')"
  C_OFF="$(printf '\033[0m')";  C_B="$(printf '\033[1m')"
else
  C_CYAN=; C_VIO=; C_DIM=; C_RED=; C_OFF=; C_B=
fi
say()  { printf '%s\n' "$*"; }
info() { printf '%s==>%s %s\n' "$C_CYAN" "$C_OFF" "$*"; }
warn() { printf '%swarning:%s %s\n' "$C_VIO" "$C_OFF" "$*" >&2; }
err()  { printf '%serror:%s %s\n' "$C_RED" "$C_OFF" "$*" >&2; exit 1; }

printf '%s%sk%sla%sv%syn%s installer\n\n' "$C_B" "$C_CYAN" "$C_OFF$C_B" "$C_VIO" "$C_OFF$C_B" "$C_OFF"

have() { command -v "$1" >/dev/null 2>&1; }

# --- detect platform -------------------------------------------------------
os="$(uname -s)"; arch="$(uname -m)"
case "$os" in
  Darwin) OS=apple-darwin ;;
  Linux)  OS=unknown-linux-gnu ;;
  *) err "unsupported OS: $os (klavyn supports macOS and Linux)" ;;
esac
case "$arch" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=aarch64 ;;
  *) err "unsupported architecture: $arch" ;;
esac
TARGET="${ARCH}-${OS}"
info "platform: ${C_B}${TARGET}${C_OFF}"

# --- fetch helper ----------------------------------------------------------
if have curl; then
  dl() { curl -fsSL "$1" -o "$2"; }
  head_ok() { curl -fsSL -o /dev/null "$1" 2>/dev/null; }
elif have wget; then
  dl() { wget -qO "$2" "$1"; }
  head_ok() { wget -q -O /dev/null "$1" 2>/dev/null; }
else
  err "need curl or wget to download"
fi

mkdir -p "$BIN_DIR"

# --- 1) try a prebuilt release --------------------------------------------
ASSET="klavyn-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
info "looking for a prebuilt release (${ASSET})..."
if head_ok "$URL"; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT INT TERM
  dl "$URL" "$tmp/$ASSET" || err "download failed"
  tar -xzf "$tmp/$ASSET" -C "$tmp" || err "extract failed"
  found="$(find "$tmp" -type f -name "$BIN_NAME" | head -n1)"
  [ -n "$found" ] || err "binary not found in release archive"
  install -m 0755 "$found" "$BIN_DIR/$BIN_NAME"
  info "installed prebuilt binary to ${C_B}${BIN_DIR}/${BIN_NAME}${C_OFF}"
else
  # --- 2) fall back to building from source --------------------------------
  warn "no prebuilt binary for ${TARGET} yet — building from source"
  have cargo || err "cargo (Rust) is required to build from source.
  Install Rust from https://rustup.rs then re-run this script."
  if [ "$os" = "Linux" ]; then
    warn "Linux build needs X11 dev libraries (Debian/Ubuntu:
  sudo apt-get install libxi-dev libxtst-dev libx11-dev)"
  fi
  info "cargo install --git https://github.com/${REPO} ${BIN_NAME} ..."
  CARGO_INSTALL_ROOT="$(mktemp -d)"
  export CARGO_INSTALL_ROOT
  cargo install --git "https://github.com/${REPO}" "$BIN_NAME" \
    || err "cargo build failed"
  install -m 0755 "$CARGO_INSTALL_ROOT/bin/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
  rm -rf "$CARGO_INSTALL_ROOT"
  info "built and installed to ${C_B}${BIN_DIR}/${BIN_NAME}${C_OFF}"
fi

# --- PATH check ------------------------------------------------------------
case ":$PATH:" in
  *":$BIN_DIR:"*) : ;;
  *) warn "$BIN_DIR is not on your PATH. Add this to your shell profile:
  export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

# --- done ------------------------------------------------------------------
say ""
info "${C_B}done.${C_OFF} next steps:"
say "  ${C_DIM}# see it recognize a chord (no corrections written):${C_OFF}"
say "  ${BIN_NAME} --dry-run"
say ""
case "$os" in
  Darwin) say "  ${C_DIM}macOS will prompt for Input Monitoring + Accessibility on first run.${C_OFF}" ;;
esac
say "  Learn chorded typing:  ${C_CYAN}https://klavyn.app/learn${C_OFF}"
say "  Docs & configuration:  ${C_CYAN}https://klavyn.app/docs${C_OFF}"
say ""
