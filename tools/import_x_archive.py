#!/usr/bin/env python3
"""Turn your own X (Twitter) data export into private voice-dataset JSONL.

    python3 tools/import_x_archive.py ~/Downloads/twitter-2026-11-24.zip
    python3 tools/import_x_archive.py path/to/extracted-archive/ --out diogenes/private/x.jsonl

Reads data/tweets.js (or data/tweet.js in older exports) from the zip or the
extracted folder and writes one line per post, following
diogenes/dataset.schema.json:

    {"prompt": "write a post", "response": "...", "source": "x:<id>"}

Replies get the prompt "reply to @<name>". Retweets are skipped. t.co links are
expanded; media links are removed. Output goes to diogenes/private/ by default,
which is gitignored: your archive never leaves your machine through this repo.
Standard library only.
"""

import argparse
import html
import json
import re
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUT = ROOT / "diogenes" / "private" / "x.jsonl"
CANDIDATES = ("data/tweets.js", "data/tweet.js")


def read_tweets_js(archive: Path) -> str:
    if archive.is_dir():
        for name in CANDIDATES:
            if (archive / name).is_file():
                return (archive / name).read_text(encoding="utf-8")
    elif zipfile.is_zipfile(archive):
        with zipfile.ZipFile(archive) as z:
            names = z.namelist()
            for name in CANDIDATES:
                match = next((n for n in names if n.endswith(name)), None)
                if match:
                    return z.read(match).decode("utf-8")
    raise SystemExit(f"no data/tweets.js found in {archive}")


def parse(js: str) -> list:
    # "window.YTD.tweets.part0 = [ ... ]"
    start = js.index("[")
    return json.loads(js[start:])


def clean(tweet: dict) -> str:
    text = tweet.get("full_text") or tweet.get("text") or ""
    for url in tweet.get("entities", {}).get("urls", []):
        if url.get("url") and url.get("expanded_url"):
            text = text.replace(url["url"], url["expanded_url"])
    for media in tweet.get("extended_entities", tweet.get("entities", {})).get("media", []):
        if media.get("url"):
            text = text.replace(media["url"], "")
    text = re.sub(r"https://t\.co/\w+", "", text)
    return html.unescape(text).strip()


def convert(items: list):
    for item in items:
        tweet = item.get("tweet", item)
        text = clean(tweet)
        if not text or text.startswith("RT @") or tweet.get("retweeted"):
            continue
        reply_to = tweet.get("in_reply_to_screen_name")
        if reply_to:
            text = re.sub(r"^(@\w+\s+)+", "", text)
        prompt = f"reply to @{reply_to}" if reply_to and tweet.get("in_reply_to_status_id_str") else "write a post"
        yield {"prompt": prompt, "response": text, "source": f"x:{tweet['id_str']}"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("archive", type=Path, help="X export .zip or extracted folder")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help=f"JSONL output (default {DEFAULT_OUT.relative_to(ROOT)})")
    args = parser.parse_args()

    out = args.out.resolve()
    private = (ROOT / "diogenes" / "private").resolve()
    if private not in out.parents and ROOT in out.parents:
        print(f"refusing to write inside the repo outside diogenes/private/: {out}", file=sys.stderr)
        return 2
    out.parent.mkdir(parents=True, exist_ok=True)

    rows = list(convert(parse(read_tweets_js(args.archive))))
    with out.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    replies = sum(r["prompt"].startswith("reply") for r in rows)
    print(f"wrote {len(rows)} lines ({len(rows) - replies} posts, {replies} replies) to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
