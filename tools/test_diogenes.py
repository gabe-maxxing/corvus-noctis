import io
import json
import threading
import unittest
from unittest.mock import patch
import urllib.error
from diogenes_bridge import Bridge, Protocol, ask, safe_reply, system_prompt, PERSONA


class DiogenesTests(unittest.TestCase):
    def test_split_and_bounded_frames(self):
        questions = []
        p = Protocol(questions.append)
        output = b"".join(p.feed(bytes([b])) for b in b"hello\x1eDIO why?\r\nworld")
        self.assertEqual(output, b"helloworld")
        self.assertEqual(questions, ["why?"])
        p.feed(b"\x1eDIO " + b"x" * 300 + b"\n")
        p.feed(b"\x1ewrong\n")
        self.assertEqual(questions, ["why?"])

    def test_persona_and_local_request(self):
        self.assertIn("Diogenes of Sinope", system_prompt(PERSONA))
        response = io.BytesIO(json.dumps({"message": {"content": "A plain life."}}).encode())
        with patch("urllib.request.build_opener") as build:
            build.return_value.open.return_value = response
            self.assertEqual(ask("why"), b"A plain life.")
            args, kwargs = build.return_value.open.call_args
            self.assertEqual(args[0].full_url, "http://127.0.0.1:11434/api/chat")
            self.assertEqual(json.loads(args[0].data)["model"], "llama3.2:3b")
            self.assertEqual(kwargs["timeout"], 35)

    def test_offline_and_control_safety(self):
        with patch("urllib.request.build_opener") as build:
            build.return_value.open.side_effect = urllib.error.URLError("offline test")
            self.assertIn(b"Install https://ollama.com", ask("why"))
        self.assertEqual(safe_reply("caf\u00e9\x1e\x1f\x1b\x03"), b"cafe")
        self.assertEqual(len(safe_reply("x" * 10000)), 2048)

    def test_cancel_drops_inflight_reply(self):
        entered, release, finished = (threading.Event() for _ in range(3))
        def oracle(text):
            entered.set(); release.wait(2); finished.set(); return b"late"
        target = io.BytesIO()
        b = Bridge(target, oracle=oracle)
        b.request("hello"); self.assertTrue(entered.wait(2))
        b.input(b"\x03"); release.set(); self.assertTrue(finished.wait(2))
        # Acquiring the generation lock makes cancellation state visible.
        with b.lock:
            self.assertEqual(b.generation, 2)
        self.assertEqual(target.getvalue(), b"\x03")


if __name__ == "__main__":
    unittest.main()
