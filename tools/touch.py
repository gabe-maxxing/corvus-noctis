"""Serve a touch interface connected to the emulated Corvus serial shell."""

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import select
import subprocess
from threading import Lock
from time import monotonic

ROOT = Path(__file__).resolve().parents[1]
PROMPT = b"corvus> "
ANSI = re.compile(rb"\x1b\[[0-9;]*[A-Za-z]")


def read_prompt(stream):
    data = bytearray()
    deadline = monotonic() + 5
    while not data.endswith(PROMPT):
        ready, _, _ = select.select([stream], [], [], max(0, deadline - monotonic()))
        if not ready:
            raise TimeoutError("The kernel did not return a prompt.")
        chunk = os.read(stream.fileno(), 4096)
        if not chunk:
            raise EOFError("The emulator exited.")
        data.extend(chunk)
    return ANSI.sub(b"", data).decode("ascii", errors="replace")


def serve(vm, bind, port):
    lock = Lock()
    banner = read_prompt(vm.stdout)

    class Handler(BaseHTTPRequestHandler):
        def reply(self, status, media_type, content):
            self.send_response(status)
            self.send_header("Content-Type", media_type)
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)

        def do_GET(self):
            if self.path == "/":
                self.reply(200, "text/html; charset=utf-8", (ROOT / "tools/touch.html").read_bytes())
            elif self.path == "/boot":
                self.reply(200, "text/plain; charset=utf-8", banner.encode())
            elif self.path == "/corvus.gif" and (ROOT / "assets/corvus.gif").is_file():
                self.reply(200, "image/gif", (ROOT / "assets/corvus.gif").read_bytes())
            else:
                self.send_error(404)

        def do_POST(self):
            if self.path != "/command":
                self.send_error(404)
                return
            if self.headers.get_content_type() != "application/json":
                self.send_error(415)
                return
            try:
                size = int(self.headers.get("Content-Length", "0"))
                if not 1 <= size <= 1024:
                    raise ValueError
                line = json.loads(self.rfile.read(size))
                if not isinstance(line, str) or not re.fullmatch(r"[ -~]{1,128}", line):
                    raise ValueError
            except (ValueError, UnicodeError):
                self.send_error(400, "Enter 1 to 128 printable ASCII characters.")
                return
            try:
                with lock:
                    vm.stdin.write(line.encode("ascii") + b"\n")
                    vm.stdin.flush()
                    response = read_prompt(vm.stdout)
            except (OSError, EOFError):
                self.send_error(502, "The emulator is unavailable.")
                return
            self.reply(200, "text/plain; charset=utf-8", response.encode())

    with ThreadingHTTPServer((bind, port), Handler) as server:
        print(f"Corvus console listening on {bind}:{server.server_port}", flush=True)
        server.serve_forever()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8080)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)
    kernel = ROOT / "target/aarch64-unknown-none-softfloat/release/corvus-os"
    vm = subprocess.Popen(
        ["qemu-system-aarch64", "-machine", "virt", "-cpu", "cortex-a57", "-nic", "none", "-display", "none",
         "-serial", "stdio", "-monitor", "none", "-kernel", str(kernel)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    )
    try:
        serve(vm, args.bind, args.port)
    except KeyboardInterrupt:
        pass
    finally:
        vm.terminate()
        vm.wait()


if __name__ == "__main__":
    main()
