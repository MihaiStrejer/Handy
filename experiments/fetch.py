"""Fetch the model and runtime revisions used by this experiment."""

import hashlib
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parent
HEADERS = {"User-Agent": "Handy-s1-mini-experiment"}
MODEL_REVISION = "34add00a48a2e5d24e5a4ee5405a99620a3a240c"
MODEL_NAME = "s1-mini-q4_k_m.gguf"
MODEL_SIZE = 484_219_808
MODEL_SHA256 = "3b41ebe2502cbd03e811d5d16b022f5ab551eda58d62597d152f89535003c634"
RUNTIME_NAME = "llama-b11200-bin-win-cpu-arm64.zip"
RUNTIME_SIZE = 12_039_946
RUNTIME_SHA256 = "ef7f4d38ed04c14eb2db7016952a2cc072ef0fea936fe00524c1f4b91bbbf4ad"
OPENCL_NAME = "llama-b11200-bin-win-opencl-adreno-arm64.zip"
OPENCL_SIZE = 12_857_356
OPENCL_SHA256 = "18b734f61e9033bfbeefa8dc96cfcd279f0e25517f1f4b542246ee996d3e9c59"


def request(url):
    return urllib.request.Request(url, headers=HEADERS)


def download(url, path, expected_sha256=None, expected_size=None):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and expected_size and path.stat().st_size != expected_size:
        path.unlink()
    if not path.exists():
        with urllib.request.urlopen(request(url), timeout=60) as response, path.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if expected_sha256 and digest != expected_sha256:
        raise ValueError(f"SHA256 mismatch for {path}: got {digest}, expected {expected_sha256}")
    print(f"{path.name}: {path.stat().st_size} bytes, SHA256 {digest}", flush=True)


model_url = f"https://huggingface.co/superwhisper/s1-mini-GGUF/resolve/{MODEL_REVISION}/{MODEL_NAME}"
runtime_url = f"https://github.com/ggml-org/llama.cpp/releases/download/b11200/{RUNTIME_NAME}"
opencl_url = f"https://github.com/ggml-org/llama.cpp/releases/download/b11200/{OPENCL_NAME}"
print(f"model revision: {MODEL_REVISION}; llama.cpp release: b11200", flush=True)
download(model_url, ROOT / "model" / MODEL_NAME, MODEL_SHA256, MODEL_SIZE)
download(runtime_url, ROOT / "downloads" / RUNTIME_NAME, RUNTIME_SHA256, RUNTIME_SIZE)
download(opencl_url, ROOT / "downloads" / OPENCL_NAME, OPENCL_SHA256, OPENCL_SIZE)
