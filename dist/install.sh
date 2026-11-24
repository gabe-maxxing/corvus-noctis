#!/bin/sh
# Corvus OS installer for Linux and other Unix systems.
#   curl -fsSL https://gabe-maxxing.github.io/corvus-noctis/install.sh | sh
#
# What it does, nothing more:
#   1. installs QEMU (qemu-system-aarch64) with your package manager, if missing
#   2. downloads the latest Corvus kernel to ~/.local/share/corvus/
#   3. writes a `corvus` command to ~/.local/bin/
# Corvus runs inside a VM window. Your disk and your OS are not touched.
# Uninstall: rm -rf ~/.local/share/corvus ~/.local/bin/corvus
set -eu

CORVUS_KERNEL_URL="${CORVUS_KERNEL_URL:-https://github.com/gabe-maxxing/corvus-noctis/releases/download/nightly/corvus-os-aarch64.elf}"
CORVUS_SOURCE_URL="${CORVUS_SOURCE_URL:-https://raw.githubusercontent.com/gabe-maxxing/corvus-noctis/main}"
SHARE="${XDG_DATA_HOME:-$HOME/.local/share}/corvus"
BIN="$HOME/.local/bin"

say() { printf '\033[1;35mcorvus\033[0m %s\n' "$*"; }
die() { printf '\033[1;31mcorvus\033[0m %s\n' "$*" >&2; exit 1; }

sudo_cmd() {
  if [ "$(id -u)" -eq 0 ]; then "$@"; elif command -v sudo >/dev/null 2>&1; then sudo "$@"; elif command -v doas >/dev/null 2>&1; then doas "$@"; else die "need root to install QEMU: run '$*' yourself, then rerun this"; fi
}

install_qemu() {
  if command -v qemu-system-aarch64 >/dev/null 2>&1; then
    say "QEMU found: $(qemu-system-aarch64 --version | head -n 1)"
    return
  fi
  say "installing QEMU (qemu-system-aarch64)..."
  if command -v apt-get >/dev/null 2>&1; then sudo_cmd apt-get update -q && sudo_cmd apt-get install -y qemu-system-arm
  elif command -v dnf >/dev/null 2>&1; then sudo_cmd dnf install -y qemu-system-aarch64
  elif command -v pacman >/dev/null 2>&1; then sudo_cmd pacman -S --needed --noconfirm qemu-system-aarch64 qemu-ui-gtk
  elif command -v zypper >/dev/null 2>&1; then sudo_cmd zypper --non-interactive install qemu-arm
  elif command -v apk >/dev/null 2>&1; then sudo_cmd apk add qemu-system-aarch64 qemu-ui-gtk
  elif command -v xbps-install >/dev/null 2>&1; then sudo_cmd xbps-install -Sy qemu
  elif command -v brew >/dev/null 2>&1; then brew install qemu
  elif command -v pkg >/dev/null 2>&1; then sudo_cmd pkg install -y qemu
  else die "no known package manager; install qemu-system-aarch64 yourself, then rerun this"
  fi
  command -v qemu-system-aarch64 >/dev/null 2>&1 || die "QEMU install finished but qemu-system-aarch64 is not on PATH"
}

fetch_kernel() {
  mkdir -p "$SHARE"
  say "downloading the Corvus kernel..."
  if command -v curl >/dev/null 2>&1; then curl -fsSL "$CORVUS_KERNEL_URL" -o "$SHARE/corvus-os.elf.part"
  elif command -v wget >/dev/null 2>&1; then wget -qO "$SHARE/corvus-os.elf.part" "$CORVUS_KERNEL_URL"
  else die "need curl or wget"
  fi
  mv "$SHARE/corvus-os.elf.part" "$SHARE/corvus-os.elf"
}

fetch_bridge() {
  for item in tools/diogenes_bridge.py diogenes/DIOGENES.md; do
    name="${item##*/}"
    if command -v curl >/dev/null 2>&1; then curl -fsSL "$CORVUS_SOURCE_URL/$item" -o "$SHARE/$name.part"
    else wget -qO "$SHARE/$name.part" "$CORVUS_SOURCE_URL/$item"
    fi
    mv "$SHARE/$name.part" "$SHARE/$name"
  done
}

write_launcher() {
  mkdir -p "$BIN"
  cat > "$BIN/corvus" <<'LAUNCHER'
#!/bin/sh
# corvus: boot Corvus OS in a QEMU window; the shell is on this terminal too.
#   corvus           boot (CORVUS_DISPLAY=none for no window)
#   corvus update    rerun the installer to fetch the newest kernel
set -eu
if [ "${1:-}" = "update" ]; then
  curl -fsSL https://gabe-maxxing.github.io/corvus-noctis/install.sh | sh
  exit
fi
kernel="${XDG_DATA_HOME:-$HOME/.local/share}/corvus/corvus-os.elf"
[ -f "$kernel" ] || { echo "corvus: kernel missing, run: corvus update" >&2; exit 1; }
if [ -n "${CORVUS_DISPLAY:-}" ]; then set -- -display "$CORVUS_DISPLAY" "$@"; fi
echo "corvus: booting. Type in this terminal or click HELP in the window. Quit: close the window or Ctrl-C."
command -v python3 >/dev/null 2>&1 || { echo "Diogenes needs Python 3. Install python3, then run corvus again."; exit 1; }
exec python3 "$(dirname "$kernel")/diogenes_bridge.py" --persona "$(dirname "$kernel")/DIOGENES.md" -- \
  qemu-system-aarch64 -machine virt -cpu cortex-a57 -nic none \
  -serial stdio -monitor none -device ramfb \
  -global virtio-mmio.force-legacy=false -device virtio-tablet-device \
  "$@" -kernel "$kernel"
LAUNCHER
  chmod +x "$BIN/corvus"
}

main() {
  say "Corvus OS installer (runs in a VM; nothing on your disk is changed)"
  install_qemu
  fetch_kernel
  fetch_bridge
  write_launcher
  say "installed: $BIN/corvus"
  case ":$PATH:" in
    *":$BIN:"*) ;;
    *) say "add ~/.local/bin to your PATH: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.profile" ;;
  esac
  say "now run: corvus"
}

main "$@"
