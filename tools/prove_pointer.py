"""Acceptance check: the on-screen HELP button runs the same command as
typing `help` over serial.

    python3 tools/prove_pointer.py

Boots the release kernel headless (ramfb + virtio tablet), types `help` on
serial, then clicks the HELP button with QMP absolute pointer events
(input-send-event), and compares both outputs byte for byte. Saves QEMU
screendumps to target/proof/. Exit code 0 means PASS.
"""

import json
import os
import re
import select
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KERNEL = ROOT / "target/aarch64-unknown-none-softfloat/release/corvus-os"
PROOF = ROOT / "target/proof"
PROMPT = b"corvus> "
ANSI = re.compile(rb"\x1b\[[0-9;]*[A-Za-z]")
WIDTH, HEIGHT = 800, 600
HELP_BUTTON = (572, 16, 212, 120)  # keep in sync with src/console.rs


def read_until(stream, marker, timeout=10):
    data = bytearray()
    deadline = time.monotonic() + timeout
    while not data.endswith(marker):
        ready, _, _ = select.select([stream], [], [], max(0, deadline - time.monotonic()))
        if not ready:
            raise TimeoutError(f"no {marker!r}; got {bytes(data)!r}")
        chunk = os.read(stream.fileno(), 4096)
        if not chunk:
            raise EOFError("QEMU exited")
        data.extend(chunk)
    return bytes(data)


class Qmp:
    def __init__(self, path):
        self.sock = socket.socket(socket.AF_UNIX)
        for _ in range(100):
            try:
                self.sock.connect(path)
                break
            except OSError:
                time.sleep(0.05)
        self.file = self.sock.makefile("rwb")
        self.file.readline()  # greeting
        self.call("qmp_capabilities")

    def call(self, command, **arguments):
        message = {"execute": command}
        if arguments:
            message["arguments"] = arguments
        self.file.write(json.dumps(message).encode() + b"\n")
        self.file.flush()
        while True:
            reply = json.loads(self.file.readline())
            if "event" in reply:
                continue
            if "error" in reply:
                raise RuntimeError(reply["error"])
            return reply["return"]


def command_output(transcript):
    """Text between the echoed `help` line and the next prompt."""
    clean = ANSI.sub(b"", transcript).replace(b"\r", b"")
    after = clean.split(b"help\n", 1)[1]
    return after[: after.rindex(PROMPT)]


def main():
    subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)
    PROOF.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rustc", "--edition", "2021", "--test", "tools/shell_tests.rs", "-D", "warnings", "-o", "target/shell-tests"], cwd=ROOT, check=True)
    subprocess.run([str(ROOT / "target/shell-tests")], check=True)
    subprocess.run([sys.executable, "-m", "unittest", "discover", "-s", "tools", "-p", "test_diogenes.py"], cwd=ROOT, check=True)
    qmp_path = os.path.join(tempfile.mkdtemp(), "qmp.sock")
    vm = subprocess.Popen(
        [str(ROOT / "tools/qemu.sh"), str(KERNEL), "-qmp", f"unix:{qmp_path},server=on,wait=off"],
        env={**os.environ, "CORVUS_DISPLAY": "none"},
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    )
    try:
        boot = read_until(vm.stdout, PROMPT)
        print(ANSI.sub(b"", boot).decode().replace("\r", "").rstrip())
        qmp = Qmp(qmp_path)
        qmp.call("screendump", filename=str(PROOF / "1-boot.ppm"))

        vm.stdin.write(b"help\r")
        vm.stdin.flush()
        typed = read_until(vm.stdout, PROMPT)

        bx, by, bw, bh = HELP_BUTTON
        cx, cy = bx + bw // 2, by + bh // 2
        ax, ay = cx * 0x8000 // WIDTH, cy * 0x8000 // HEIGHT
        qmp.call("input-send-event", events=[
            {"type": "abs", "data": {"axis": "x", "value": ax}},
            {"type": "abs", "data": {"axis": "y", "value": ay}},
        ])
        qmp.call("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": "left"}}])
        pointer = read_until(vm.stdout, PROMPT)  # serial trace + prompt
        if pointer.count(PROMPT) < 2:  # both prompts can arrive in one read
            pointer += read_until(vm.stdout, PROMPT)  # echoed help + output
        qmp.call("screendump", filename=str(PROOF / "2-help-pressed.ppm"))
        qmp.call("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": "left"}}])
        time.sleep(0.3)
        qmp.call("screendump", filename=str(PROOF / "3-after-release.ppm"))
        vm.stdin.write(b"dio why\r"); vm.stdin.flush()
        assert b"\x1eDIO why\n" in read_until(vm.stdout, b"\x1eDIO why\n")
        vm.stdin.write(b"\x1eLive plainly.\x1f"); vm.stdin.flush()
        assert b"Live plainly." in read_until(vm.stdout, PROMPT)
        vm.stdin.write(b"dio\r"); vm.stdin.flush()
        read_until(vm.stdout, b"diogenes> ")
        vm.stdin.write(b"? virtues\r"); vm.stdin.flush()
        read_until(vm.stdout, b"\x1eDIO virtues\n")
        vm.stdin.write(b"\x03"); vm.stdin.flush()
        assert b"^C" in read_until(vm.stdout, b"diogenes> ")
        vm.stdin.write(b"\x1elate answer\x1fexit\r"); vm.stdin.flush()
        assert b"late answer" not in read_until(vm.stdout, PROMPT)
        print("PASS: kernel Diogenes framing, mode, cancellation and late reply isolation")
        vm.stdin.write(b'i=0; while(i<5){Draw(i*20,0,15,15,14); i=i+1}; i\r'); vm.stdin.flush()
        assert b"5\r\n" in read_until(vm.stdout, PROMPT)
        vm.stdin.write(b"while(1){}\r"); vm.stdin.flush()
        assert b"step limit reached" in read_until(vm.stdout, PROMPT)
        vm.stdin.write(b'if(1){\r'); vm.stdin.flush()
        read_until(vm.stdout, b".. ")
        vm.stdin.write(b'\x03calc 6*7\r'); vm.stdin.flush()
        response = read_until(vm.stdout, PROMPT)
        if b"42\r\n" not in response: response += read_until(vm.stdout, PROMPT)
        assert b"42\r\n" in response
        print("PASS: kernel CrowC drawing loop, step limit, continuation cancellation and legacy recovery")
    finally:
        vm.terminate()
        vm.wait()

    print("--- typed over serial ---")
    print(ANSI.sub(b"", typed).decode().replace("\r", "").rstrip())
    print("--- HELP button via QMP pointer ---")
    print(ANSI.sub(b"", pointer).decode().replace("\r", "").rstrip())
    a, b = command_output(typed), command_output(pointer)
    same = a == b and b"[pointer] HELP pressed" in pointer
    print(f"--- {'PASS' if same else 'FAIL'}: pointer output {'==' if a == b else '!='} serial output ({len(a)} bytes)")
    print(f"screendumps: {PROOF}")
    return 0 if same else 1


if __name__ == "__main__":
    sys.exit(main())
