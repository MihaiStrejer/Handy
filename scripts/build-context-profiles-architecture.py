"""Build the offline architecture walkthrough: uv run scripts/build-context-profiles-architecture.py."""

import base64
import hashlib
import html
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROOF = ROOT / "design/proof/architecture"
OUTPUT = ROOT / "design/context-profiles-architecture.html"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


SOURCES = [
    ("E1", "Startup managers and shared state", "src-tauri/src/lib.rs", "context_profiles::storage::ProfileMemory::default()", 9, 12),
    ("E2", "Shortcut gate and target capture", "src-tauri/src/actions.rs", ".then(capture_target)", 16, 20),
    ("E3", "Background capture and session identity", "src-tauri/src/context_profiles/mod.rs", "let ticket =", 5, 24),
    ("E4", "Deterministic routing and inherited prompt", "src-tauri/src/context_profiles/routing.rs", "let (profile, match_basis)", 4, 26),
    ("E5", "Template field references", "src-tauri/src/context_profiles/template.rs", "pub(super) fn render", 3, 21),
    ("E6", "Actual request envelope", "src-tauri/src/context_profiles/request.rs", "fn assemble(", 0, 23),
    ("E7", "Strict reply parsing", "src-tauri/src/context_profiles/request.rs", "fn parse(", 0, 27),
    ("E8", "Compatibility probe cache", "src-tauri/src/context_profiles/request.rs", "pub(crate) async fn ensure_compatible", 0, 39),
    ("E9", "Response, input recheck and session acceptance", "src-tauri/src/context_profiles/request.rs", "let content = send(settings, system, user)", 0, 24),
    ("E10", "Memory storage and exposed operations", "src-tauri/src/context_profiles/storage.rs", "pub(crate) struct ProfileMemory", 2, 36),
    ("E11", "Learning remains blocked in the task plan", "plan.md", "## T07:", 0, 16),
    ("E12", "Cleanup and cancellation", "src-tauri/src/context_profiles/session.rs", "pub(crate) fn finish", 0, 18),
    ("E13", "Verification evidence and remaining gates", "plan.md", "## T09:", 0, 19),
    ("E14", "Final session and focus checks before paste", "src-tauri/src/actions.rs", "let context_for_paste =", 1, 39),
    ("E15", "Supported native Edit capture", "src-tauri/src/context_profiles/capture.rs", "fn read_control(", 0, 31),
]


def evidence():
    blocks = []
    for key, title, path, anchor, before, after in SOURCES:
        raw = (ROOT / path).read_bytes()
        lines = raw.decode("utf-8-sig").splitlines()
        index = next(i for i, line in enumerate(lines) if anchor in line)
        start, end = max(0, index - before), min(len(lines), index + after)
        excerpt = "\n".join(f"{i + 1:4}  {lines[i]}" for i in range(start, end))
        blocks.append(f'<details id="{key}"><summary>{key} · {html.escape(title)} <small>{path}:{index + 1}</small></summary><p class="caption">Working-tree source SHA-256: {hashlib.sha256(raw).hexdigest()}</p><pre>{html.escape(excerpt)}</pre></details>')
    return "\n".join(blocks)


page = (ROOT / "design/context-profiles-architecture.template.html").read_text(encoding="utf-8")
capture = json.loads((PROOF / "capture.json").read_text(encoding="utf-8-sig"))
replacements = {
    "@@COMMIT@@": html.escape(git("rev-parse", "--short", "HEAD")),
    "@@BRANCH@@": html.escape(git("branch", "--show-current")),
    "@@BUILT@@": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    "@@CAPTURE@@": html.escape(capture["captured_at"]),
    "@@EVIDENCE@@": evidence(),
}
for name in ("context-original", "dictionary-annotated", "overlay"):
    replacements[f"@@{name.upper()}@@"] = "data:image/png;base64," + base64.b64encode((PROOF / f"{name}.png").read_bytes()).decode("ascii")
for token, value in replacements.items():
    page = page.replace(token, value)
if "@@" in page:
    raise ValueError("Unresolved build placeholder")
OUTPUT.write_text(page, encoding="utf-8")
print(f"Built {OUTPUT} ({OUTPUT.stat().st_size:,} bytes)")
