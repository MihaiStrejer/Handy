"""Test full-output cleanup of a varied synthetic transcript in the warm Codex session."""

import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


RESULTS = Path(__file__).resolve().parent / "results"
previous = json.loads((RESULTS / "measurements-codex-luna.json").read_text(encoding="utf-8"))
thread_id = previous["cases"][-1]["thread_id"]
codex = shutil.which("codex")
if codex is None:
    raise RuntimeError("codex CLI is not on PATH")

items = []
for number in range(1, 81):
    items.append(
        f"item {number}: so um Mara checked version {number} of the route map on Tuesday, "
        f"and Jules will send the revised notes to team {number} before Thursday; "
        f"the budget for that item is {number * 37} dollars and the owner is group {number % 7 + 1}."
    )
prompt = (
    "Clean this dictated transcript. Keep all 80 numbered items, their order, names, numbers, dates, and amounts. "
    "Remove only filler words and repair punctuation. Output all 80 cleaned items with their numbers. "
    "Do not summarize or use tools.\n\n" + "\n".join(items)
)
command = [
    codex, "exec", "resume", "--skip-git-repo-check", "--ignore-user-config",
    "-m", "gpt-6-luna", "-c", 'model_reasoning_effort="none"',
    "--json", thread_id, "-",
]
start = time.perf_counter()
result = subprocess.run(command, input=prompt, capture_output=True, text=True, cwd=tempfile.gettempdir(), timeout=600)
elapsed = time.perf_counter() - start
(RESULTS / "codex-luna-diverse.jsonl").write_text(result.stdout, encoding="utf-8")
(RESULTS / "codex-luna-diverse.stderr.txt").write_text(result.stderr, encoding="utf-8")
events = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
completed = next((event for event in events if event.get("type") == "turn.completed"), None)
messages = [event["item"]["text"] for event in events if event.get("type") == "item.completed" and event.get("item", {}).get("type") == "agent_message"]
output = messages[-1] if messages else ""
(RESULTS / "codex-luna-diverse-output.txt").write_text(output, encoding="utf-8")
record = {
    "seconds": round(elapsed, 3),
    "exit_code": result.returncode,
    "input_words": len(prompt.split()),
    "output_words": len(output.split()),
    "numbered_items_in_output": sum(f"{number}." in output or f"Item {number}:" in output for number in range(1, 81)),
    "usage": completed.get("usage") if completed else None,
    "output_excerpt": output[:250],
}
(RESULTS / "measurements-codex-luna-diverse.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
print(json.dumps(record), flush=True)
if result.returncode != 0 or completed is None:
    raise RuntimeError("diverse transcript request failed; see results/codex-luna-diverse logs")
