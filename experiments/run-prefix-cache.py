"""Measure reuse of a large system-message prefix with new user inputs."""

import json
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SERVER = ROOT / "runtime_opencl" / "llama-server.exe"
MODEL = ROOT / "model" / "s1-mini-q4_k_m.gguf"
RESULTS = ROOT / "results"
RESULTS.mkdir(exist_ok=True)
PORT = 18492
BASE = f"http://127.0.0.1:{PORT}"
SYSTEM = "You are a text normalizer for speech-to-text transcripts. The input begins with a control line specifying the styling, structure, and context settings; clean the transcript to match those settings and output only the cleaned text."
CONTROL = "[Styling: semi-formal] [Structure: prose] [Context: general]"
PREFIX_SENTENCE = "The reference notes describe the project vocabulary and preferred wording for each speaker."
WARM_SENTENCE = "I will send the revised report on Thursday."
TWO_K_SENTENCE = "The team reviewed the draft, revised the schedule, and approved the final report."
THREE_K_SENTENCE = "After the meeting, we updated the document and shared the next steps with everyone."
STATIC_SYSTEM = SYSTEM + "\nReference notes:\n" + " ".join([PREFIX_SENTENCE] * 995)


def request(path, payload=None, timeout=600):
    body = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(BASE + path, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return json.load(response)


def working_set(pid):
    result = subprocess.run(
        ["powershell", "-NoProfile", "-Command", f"(Get-Process -Id {pid}).WorkingSet64"],
        capture_output=True, text=True, check=True,
    )
    return int(result.stdout.strip())


def chat_payload(transcript, cache_prompt=True):
    return {
        "messages": [
            {"role": "system", "content": STATIC_SYSTEM},
            {"role": "user", "content": CONTROL + "\n" + transcript},
        ],
        "temperature": 0,
        "max_tokens": 1,
        "cache_prompt": cache_prompt,
    }


args = [
    str(SERVER), "-m", str(MODEL), "--host", "127.0.0.1", "--port", str(PORT),
    "-c", "20480", "-np", "1", "-t", "12", "-ngl", "99", "--jinja",
    "--chat-template-kwargs", '{"enable_thinking":false}', "--temp", "0",
]
log = (RESULTS / "server-opencl-prefix-cache.log").open("w", encoding="utf-8")
server = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
record = {"command": args, "pid": server.pid, "cases": [], "prefix_repetitions": 995}
try:
    start = time.perf_counter()
    for _ in range(240):
        if server.poll() is not None:
            raise RuntimeError(f"server exited with code {server.returncode}; see {log.name}")
        try:
            if request("/health", timeout=1).get("status") == "ok":
                break
        except (OSError, urllib.error.URLError, ValueError):
            pass
        time.sleep(0.5)
    else:
        raise TimeoutError("server did not become healthy within 120 seconds")
    record["startup_seconds"] = round(time.perf_counter() - start, 3)
    record["working_set_after_load_bytes"] = working_set(server.pid)

    cases = [
        ("warm_prefix", WARM_SENTENCE, True),
        ("new_2k_cached", " ".join([TWO_K_SENTENCE] * 125), True),
        ("new_3k_cached", " ".join([THREE_K_SENTENCE] * 185), True),
        ("new_2k_uncached", " ".join([TWO_K_SENTENCE] * 125), False),
    ]
    for name, transcript, cache_prompt in cases:
        payload = chat_payload(transcript, cache_prompt)
        start = time.perf_counter()
        try:
            response = request("/v1/chat/completions", payload)
            case = {
                "name": name,
                "input_words": len(transcript.split()),
                "seconds": round(time.perf_counter() - start, 3),
                "working_set_bytes": working_set(server.pid),
                "usage": response.get("usage"),
                "timings": response.get("timings"),
                "finish_reason": response["choices"][0]["finish_reason"],
                "output": response["choices"][0]["message"]["content"],
            }
        except Exception as exc:
            case = {"name": name, "seconds": round(time.perf_counter() - start, 3), "error": repr(exc)}
            record["cases"].append(case)
            print(json.dumps(case), flush=True)
            raise
        record["cases"].append(case)
        print(json.dumps(case), flush=True)
finally:
    server.terminate()
    try:
        server.wait(timeout=10)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait()
    log.close()
    (RESULTS / "measurements-opencl-prefix-cache.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
