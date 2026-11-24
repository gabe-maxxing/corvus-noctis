#!/bin/sh
# Boot a Corvus OS kernel on QEMU virt: serial on stdio, ramfb display,
# virtio tablet (absolute pointer). Usage: tools/qemu.sh KERNEL [qemu args...]
# CORVUS_DISPLAY=none runs headless; any other value is passed to -display.
set -e
kernel="$1"
shift
if [ -n "${CORVUS_DISPLAY:-}" ]; then
  set -- -display "$CORVUS_DISPLAY" "$@"
fi
set -- qemu-system-aarch64 -machine virt -cpu cortex-a57 -nic none \
  -serial stdio -monitor none \
  -device ramfb \
  -global virtio-mmio.force-legacy=false -device virtio-tablet-device \
  "$@" -kernel "$kernel"

if [ -t 0 ] && [ "${CORVUS_DIO:-1}" != 0 ] && command -v python3 >/dev/null 2>&1; then
  exec python3 "$(dirname "$0")/diogenes_bridge.py" -- "$@"
fi
exec "$@"
