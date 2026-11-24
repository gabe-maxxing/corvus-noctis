#!/usr/bin/env python3
"""Local-only Ollama bridge. Run: python3 diogenes_bridge.py -- QEMU ARGS..."""
import argparse
import json
import os
from pathlib import Path
import queue
import select
import subprocess
import time
import sys
import threading
import unicodedata
import urllib.error
import urllib.request

MODEL = os.environ.get("DIOGENES_MODEL", "llama3.2:3b")
OFFLINE = "Diogenes offline: my jar has no oracle. Install https://ollama.com then run: ollama run " + MODEL
PERSONA = Path(__file__).resolve().parents[1] / "diogenes/DIOGENES.md"


def system_prompt(path):
    return Path(path).read_text(encoding="utf-8").split("## System prompt\n", 1)[1].split("\n## ", 1)[0].strip()


def safe_reply(text):
    text = unicodedata.normalize("NFKD", str(text)).encode("ascii", "ignore")
    return bytes(b for b in text if b == 10 or 32 <= b <= 126)[:2048]


def ask(text, persona=PERSONA):
    try:
        body = json.dumps({"model": MODEL, "stream": False, "messages": [
            {"role": "system", "content": system_prompt(persona)},
            {"role": "user", "content": text}], "options": {"num_predict": 256}}).encode()
        request = urllib.request.Request("http://127.0.0.1:11434/api/chat", body,
                                         {"Content-Type": "application/json"})
        # Never use HTTP_PROXY for local questions; no network listener or shell execution.
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with opener.open(request, timeout=35) as response:
            data = json.loads(response.read(65537))
        reply = data["message"]["content"]
        if not isinstance(reply, str) or not reply.strip():
            raise ValueError("empty reply")
        return safe_reply(reply)
    except (OSError, ValueError, KeyError, IndexError, TypeError, urllib.error.URLError) as error:
        print("diogenes bridge: " + str(error), file=sys.stderr)
        return safe_reply(OFFLINE)


class Protocol:
    """Bounded streaming parser; request delimiters may cross pipe reads."""
    def __init__(self, request):
        self.request = request
        self.frame = None
        self.overflow = False

    def feed(self, chunk):
        visible = bytearray()
        for byte in chunk:
            if byte == 30:
                self.frame = bytearray()
                self.overflow = False
            elif self.frame is None:
                visible.append(byte)
            elif byte == 10:
                frame, self.frame = bytes(self.frame).rstrip(b"\r"), None
                if not self.overflow and frame.startswith(b"DIO "):
                    self.request(frame[4:].decode("ascii", "replace"))
            elif len(self.frame) < 260:
                self.frame.append(byte)
            else:
                self.overflow = True
        return bytes(visible)


class Bridge:
    def __init__(self, stdin, persona=PERSONA, oracle=None):
        self.stdin, self.persona = stdin, persona
        self.oracle = oracle or (lambda text: ask(text, persona))
        self.lock = threading.Lock()
        self.generation = 0
        self.jobs = queue.Queue(maxsize=1)
        self.protocol = Protocol(self.request)
        threading.Thread(target=self.worker, daemon=True).start()

    def input(self, data):
        with self.lock:
            if b"\x03" in data:
                self.generation += 1
            self.stdin.write(data)
            self.stdin.flush()

    def request(self, text):
        with self.lock:
            self.generation += 1
            job = (self.generation, text)
            try:
                self.jobs.get_nowait()
            except queue.Empty:
                pass
            self.jobs.put_nowait(job)

    def worker(self):
        while True:
            generation, text = self.jobs.get()
            reply = safe_reply(self.oracle(text).decode("ascii", "replace"))
            with self.lock:
                if generation == self.generation:
                    try:
                        self.stdin.write(b"\x1e" + reply + b"\x1f")
                        self.stdin.flush()
                    except (OSError, ValueError):
                        return


def run(command, persona):
    vm = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0,
                          start_new_session=os.name != "nt",
                          creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
    bridge = Bridge(vm.stdin, persona)
    def output():
        while True:
            chunk = vm.stdout.read(4096)
            if not chunk:
                return
            data = bridge.protocol.feed(chunk)
            sys.stdout.buffer.write(data)
            sys.stdout.buffer.flush()
    threading.Thread(target=output, daemon=True).start()
    previous = None
    try:
        if os.name == "nt":
            import msvcrt
            while vm.poll() is None:
                if not msvcrt.kbhit():
                    time.sleep(0.05)
                    continue
                char = msvcrt.getwch()
                if char in ("\x00", "\xe0"):
                    char = {"H": "\x1b[A", "P": "\x1b[B"}.get(msvcrt.getwch(), "")
                bridge.input(char.encode("ascii", "ignore"))
        else:
            if sys.stdin.isatty():
                import termios
                import tty
                previous = termios.tcgetattr(sys.stdin)
                tty.setcbreak(sys.stdin.fileno())
            while vm.poll() is None:
                try:
                    if not select.select([sys.stdin], [], [], 0.1)[0]:
                        continue
                    data = os.read(sys.stdin.fileno(), 4096)
                    if not data:
                        break
                    bridge.input(data)
                except KeyboardInterrupt:
                    bridge.input(b"\x03")
    finally:
        if previous is not None:
            termios.tcsetattr(sys.stdin, termios.TCSADRAIN, previous)
        if vm.poll() is None:
            vm.terminate()
        vm.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--persona", type=Path, default=PERSONA)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("pass -- followed by QEMU and its arguments")
    run(command, args.persona)


if __name__ == "__main__":
    main()
