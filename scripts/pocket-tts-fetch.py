#!/usr/bin/env python3
"""Fetches Pocket TTS's weights and voices for Coupler's characters'
voices (src-tauri/src/synth.rs), checks them, and prepares what ships in
the app: src-tauri/resources/pocket-tts/ (not in git: ~250 MB).

This is build tooling, run on the developer's machine: the app itself
never downloads anything (CLAUDE.md rule 2), it reads these files from
its own Resources folder. build-release.sh runs this before every
release build; run it once yourself after cloning, before `npm run tauri
dev` (without the files, the Pocket TTS voices are simply not offered).

What it takes, all from kyutai/pocket-tts-without-voice-cloning at one
pinned revision, each file checked against its SHA-256:
- tts_b6369a24.safetensors, the model (CC BY 4.0, Kyutai). Written
  without the tensors that make a voice from a recording (the Mimi
  encoder, its transformer, the downsampler, the speaker projection):
  Coupler offers presets only, and vendor/pocket-tts can't use them.
- tokenizer.model, its SentencePiece vocabulary.
- embeddings_v3/<voice>.safetensors for the 19 English presets whose
  recordings are CC0 or CC BY 4.0 (VOICES below).
  Not cosette (Expresso) or jean (EARS): CC BY-NC. Each is written as
  16-bit floats, half the size, unless a value doesn't fit.

Downloads are kept in src-tauri/target/pocket-tts-download/ so a second
run is quick; outputs are rewritten only when the revision changes.

Usage: scripts/pocket-tts-fetch.py [--check]
  --check  only say whether the prepared files are there and current.
"""
import hashlib
import json
import struct
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "resources" / "pocket-tts"
CACHE = ROOT / "src-tauri" / "target" / "pocket-tts-download"
REPO = "kyutai/pocket-tts-without-voice-cloning"
REVISION = "1e08e6a23401048648a9fdcfde2f89348215c2a7"

MODEL = ("tts_b6369a24.safetensors", "58aa704a88faad35f22c34ea1cb55c4c5629de8b8e035c6e4936e2673dc07617")
TOKENIZER = ("tokenizer.model", "d461765ae179566678c93091c5fa6f2984c31bbe990bf1aa62d92c64d91bc3f6")
# Voice, its file's SHA-256. synth.rs's VOICES lists the same, with each
# one's name, kind and source.
VOICES = {
    "alba": "76fcc99b552439f1d2a96d593285113f6afaf628281195a2959b40f5c3610f3f",
    "anna": "613a0244f538f6916645da06de12f57389a9f95855ec9db04c8be0806ff4ddc5",
    "azelma": "90da1c3967ca9505dbe82e3388829b97ade2b9b5d48750be0cedaf6a77355f58",
    "bill_boerst": "7e77da11c6deb8bddf7b6ac1e0c9988dab36874b1432b9fb1d8a985bf7675f6e",
    "caro_davy": "9280f8e2141ce45316c2020138641a08895eb05ab53c93f0257c5b71c42f47c4",
    "charles": "595d60eccc8cd3e2018f03bd8a1e83d6ae075e6c637e38940850e76c98a3f747",
    "eponine": "c2b0ab4c86a946c0bf4a14d6ca586a9f72d9ff6911839bd7c9b3613b3a726dd6",
    "eve": "2a0501e2a90c56cb83fc5ac20796dab9d5f6189ed2f98d5cb7aa7f55047c76bf",
    "fantine": "7c07450f96a0d75b57c44d3b7254d2338b4ba5130016997168f65272dc3c6332",
    "george": "96f7947468e92920c9d6e5aeda8903e15845f0a811f4471f90fe988ebc38cd71",
    "jane": "f9cd0c17fab9aef3ff274f3e42b3679e39277ad148353c1461b4c0bcb9221bd5",
    "javert": "fce55ce88a4f8239f336b940b7bd33b97d9795975653aaff074526f0dfbea1e0",
    "marius": "2393140a1ff79fefd7ac0b70fdfa71fd7a0034255a9dc44f9931661ad70bb476",
    "mary": "5180a56946840103b0e53756cb9b53cc18e580ff68fad5a2d741dc6ce03f5da1",
    "michael": "7bc77e645e7a530484844454de003fc434c9f2dca08e88ba4bcae1e5f52b1653",
    "paul": "a5738f4593c7d2424fdcfc935e9b5fc9cf1bcfe024c4fce204f19f1346d38d2f",
    "peter_yearsley": "619f8a155ab47ac8e3556ccfa9caee0cf94997a343395d3233d7fa35c202a5d9",
    "stuart_bell": "3851de39075693911b41676ab557bc6900fdba801a03e40d140b962ab36cbbeb",
    "vera": "773e9d338148b2e428a498af3174a0b76d9ba1bfc44eac159e38a2209aa5e4cf",
}
# The tensors that turn a recording into a voice: left out.
CLONING = ("mimi.encoder.", "mimi.encoder_transformer.", "mimi.downsample.", "flow_lm.speaker_proj_weight")
STAMP = OUT / "REVISION"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def fetch(name: str, digest: str) -> Path:
    """The file, downloaded once and checked."""
    path = CACHE / name
    if path.exists() and sha256(path) == digest:
        return path
    path.parent.mkdir(parents=True, exist_ok=True)
    url = f"https://huggingface.co/{REPO}/resolve/{REVISION}/{name}"
    print(f"Downloading {name}…", flush=True)
    temp = path.with_suffix(path.suffix + ".part")
    temp.unlink(missing_ok=True)
    # A connection can end early without an error: carry on from where it
    # stopped until the whole file is here.
    total, tries = None, 0
    while total is None or temp.stat().st_size < total:
        tries += 1
        if tries > 20:
            sys.exit(f"{name} kept stopping short. Check the connection and run this again.")
        have = temp.stat().st_size if temp.exists() else 0
        request = urllib.request.Request(url, headers={"Range": f"bytes={have}-"} if have else {})
        with urllib.request.urlopen(request, timeout=60) as r, open(temp, "ab" if have else "wb") as f:
            if total is None:
                total = int(r.headers["Content-Length"])
            elif r.status != 206:
                sys.exit(f"{name}: the server won't resume a download. Run this again.")
            try:
                while block := r.read(1 << 20):
                    f.write(block)
            except OSError:
                pass  # Resumed on the next pass.
    got = sha256(temp)
    if got != digest:
        temp.unlink()
        sys.exit(f"{name} isn't the file Coupler was checked with (SHA-256 {got}). Not used.")
    temp.rename(path)
    return path


def read_safetensors(path: Path):
    data = path.read_bytes()
    n = struct.unpack("<Q", data[:8])[0]
    header = json.loads(data[8 : 8 + n])
    header.pop("__metadata__", None)
    return header, memoryview(data)[8 + n :]


def write_safetensors(path: Path, tensors: list):
    """`tensors`: (name, dtype, shape, bytes), written in that order."""
    header, offset = {}, 0
    for name, dtype, shape, raw in tensors:
        header[name] = {"dtype": dtype, "shape": shape, "data_offsets": [offset, offset + len(raw)]}
        offset += len(raw)
    text = json.dumps(header, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 8)
    temp = path.with_suffix(".part")
    with open(temp, "wb") as f:
        f.write(struct.pack("<Q", len(text)))
        f.write(text)
        for *_, raw in tensors:
            f.write(raw)
    temp.rename(path)


def prepare_model(src: Path, dest: Path):
    header, body = read_safetensors(src)
    kept = []
    for name, t in sorted(header.items(), key=lambda kv: kv[1]["data_offsets"][0]):
        if name.startswith(CLONING):
            continue
        a, b = t["data_offsets"]
        kept.append((name, t["dtype"], t["shape"], body[a:b]))
    write_safetensors(dest, kept)
    print(f"Model: {len(kept)} of {len(header)} tensors kept, {dest.stat().st_size / 1e6:.0f} MB.")


def prepare_voice(src: Path, dest: Path):
    header, body = read_safetensors(src)
    out = []
    for name, t in sorted(header.items()):
        a, b = t["data_offsets"]
        raw, dtype = body[a:b], t["dtype"]
        if dtype == "F32":
            values = struct.unpack(f"<{len(raw) // 4}f", raw)
            try:
                raw, dtype = struct.pack(f"<{len(values)}e", *values), "F16"
            except (OverflowError, struct.error):
                pass  # Doesn't fit in 16 bits: kept as it was.
        out.append((name, dtype, t["shape"], bytes(raw)))
    write_safetensors(dest, out)


def current() -> bool:
    files = [OUT / "model.safetensors", OUT / "tokenizer.model"] + [OUT / "voices" / f"{v}.safetensors" for v in VOICES]
    return STAMP.exists() and STAMP.read_text().strip() == REVISION and all(f.exists() for f in files)


def main():
    if "--check" in sys.argv:
        ok = current()
        print("Pocket TTS files are ready." if ok else "Pocket TTS files are missing or old: run scripts/pocket-tts-fetch.py.")
        sys.exit(0 if ok else 1)
    if current():
        print("Pocket TTS files are ready.")
        return
    (OUT / "voices").mkdir(parents=True, exist_ok=True)
    STAMP.unlink(missing_ok=True)
    prepare_model(fetch(*MODEL), OUT / "model.safetensors")
    (OUT / "tokenizer.model").write_bytes(fetch(*TOKENIZER).read_bytes())
    for voice, digest in VOICES.items():
        prepare_voice(fetch(f"embeddings_v3/{voice}.safetensors", digest), OUT / "voices" / f"{voice}.safetensors")
    for stale in (OUT / "voices").glob("*.safetensors"):
        if stale.stem not in VOICES:
            stale.unlink()
    STAMP.write_text(REVISION + "\n")
    total = sum(f.stat().st_size for f in OUT.rglob("*") if f.is_file())
    print(f"Pocket TTS files ready in {OUT.relative_to(ROOT)}: {total / 1e6:.0f} MB.")


if __name__ == "__main__":
    main()
