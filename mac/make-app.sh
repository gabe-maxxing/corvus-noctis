#!/bin/sh
# Builds Corvus-mac.zip: Corvus.app, which opens Terminal and boots the bundled
# Corvus kernel in a QEMU window (serial shell in Terminal, HELP button in the
# window). Needs QEMU from Homebrew; the app says so if it is missing.
#   mac/make-app.sh KERNEL_ELF OUT_DIR
# Runs on any Unix with `zip` (CI builds it on Linux). The app is unsigned:
# first launch is right-click > Open.
set -eu
kernel="$1"
mkdir -p "$2"
out="$(cd "$2" && pwd)"
work="$(mktemp -d)"
app="$work/Corvus.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$kernel" "$app/Contents/Resources/corvus-os.elf"
script_dir="$(cd "$(dirname "$0")" && pwd)"
cp "$script_dir/../docs/brand/Corvus.icns" "$app/Contents/Resources/Corvus.icns"
cp "$script_dir/../tools/diogenes_bridge.py" "$app/Contents/Resources/diogenes_bridge.py"
cp "$script_dir/../diogenes/DIOGENES.md" "$app/Contents/Resources/DIOGENES.md"

cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Corvus</string>
  <key>CFBundleDisplayName</key><string>Corvus OS</string>
  <key>CFBundleIdentifier</key><string>gent.corvus.os</string>
  <key>CFBundleVersion</key><string>0.2</string>
  <key>CFBundleShortVersionString</key><string>0.2</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>Corvus.icns</string>
  <key>CFBundleExecutable</key><string>Corvus</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict>
</plist>
PLIST

# The app's executable only opens Terminal on run.command, so the Corvus shell
# has a real terminal for its serial console.
cat > "$app/Contents/MacOS/Corvus" <<'EXEC'
#!/bin/sh
here="$(cd "$(dirname "$0")/../Resources" && pwd)"
open -a Terminal "$here/run.command"
EXEC

cat > "$app/Contents/Resources/run.command" <<'RUN'
#!/bin/sh
here="$(cd "$(dirname "$0")" && pwd)"
PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
clear
printf '\033[1;35m   .__\n   (o )>   C O R V U S\n   / )     running in a VM on your Mac\n  /_/\033[0m\n\n'
if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
  echo "Corvus needs QEMU (a free virtual machine)."
  if command -v brew >/dev/null 2>&1; then
    printf "Install it now with Homebrew? [Y/n] "
    read -r answer
    case "$answer" in n*|N*) exit 1 ;; esac
    brew install qemu || exit 1
  else
    echo "Install Homebrew from https://brew.sh, then run: brew install qemu"
    echo "Then open Corvus again."
    exit 1
  fi
fi
echo "Booting. Type here, or click HELP in the Corvus window. Quit: close the window."
if ! command -v python3 >/dev/null 2>&1; then
  echo "Diogenes needs Python 3: brew install python. Then open Corvus again."
  exit 1
fi
exec python3 "$here/diogenes_bridge.py" --persona "$here/DIOGENES.md" -- \
  qemu-system-aarch64 -machine virt -cpu cortex-a57 -nic none \
  -serial stdio -monitor none -device ramfb -display cocoa \
  -global virtio-mmio.force-legacy=false -device virtio-tablet-device \
  -kernel "$here/corvus-os.elf"
RUN

chmod +x "$app/Contents/MacOS/Corvus" "$app/Contents/Resources/run.command"
cat > "$work/README.txt" <<'README'
Corvus OS for macOS
1. Drag Corvus.app to Applications (optional).
2. First launch: right-click Corvus.app > Open (it is not signed yet).
3. If QEMU is missing, Corvus offers to install it with Homebrew.
Corvus runs inside a virtual machine window. Nothing on your Mac is changed.
README
rm -f "$out/Corvus-mac.zip"
(cd "$work" && zip -qry "$out/Corvus-mac.zip" Corvus.app README.txt)
rm -rf "$work"
echo "$out/Corvus-mac.zip"
