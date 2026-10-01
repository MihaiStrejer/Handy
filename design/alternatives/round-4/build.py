"""Build the combined review revision from the retained parts of H, I and J."""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent
ROUND3 = ROOT.parent / "round-3"
page = (ROUND3 / "i-illustrated-session/index.html").read_text(encoding="utf-8-sig")
questions = (ROUND3 / "j-reader-questions/index.html").read_text(encoding="utf-8-sig")
overview = re.search(r'<svg\b.*?</svg>', questions, re.S).group(0)
# One continuous branch, with arrowheads only where it reaches an action.
overview = overview.replace('<path class="path" d="M266 140H288"/><path class="parallel" d="M288 140V194"/><path class="path" d="M288 140V135H322"/><path class="path" d="M288 194V252H322"/>', '<path class="connector" d="M266 140H288"/><path class="path" d="M288 140V135H322"/><path class="path" d="M288 140V252H322"/>')
assert 'class="parallel"' not in overview
overview = re.sub(r' rx="\d+"', ' rx="0"', overview)
overview = overview.replace('PREPARED BEFORE YOU SPEAK', 'AT STARTUP · WHEN PROFILES ARE ENABLED').replace('Profiles · prompt · dictionary', 'Load saved profile settings').replace('Saved routing rules identify a profile', 'App rules · prompt · dictionary')
overview = overview.replace('<text class="micro" x="32" y="34">', '<text class="micro startup-label" x="32" y="34">')
overview = overview.replace('class="box-orange" x="604"', 'class="box-gray" x="604"')
hero = '''<section class="hero wrap"><h1>Handy: from recording to output</h1><p class="lede">Handy chooses a profile for the focused app, combines its instructions and vocabulary with your transcript, then checks the reply before sending text back to the original input.</p></section>'''
page, n = re.subn(r'<section class="hero wrap">.*?</section>', hero, page, count=1, flags=re.S)
assert n == 1
map_section = f'''<section class="overview" id="overview"><div class="wrap"><div class="overview-header"><h2>Recording and context meet at the request</h2><p>Starting a post-process recording begins two independent jobs: transcribing speech and identifying the profile and available input context. Both must finish before Handy sends the request. A profile is saved configuration; a session is one recording and its related work.</p></div><div class="diagram-scroll" role="region" tabindex="0" aria-label="Architecture overview; scroll horizontally on narrow screens">{overview}</div><p class="scroll-hint">Scroll horizontally to follow the whole diagram.</p><p class="map-note">Solid arrows show the current flow, not elapsed time. The two branches run independently. This is a diagram based on the code; a complete live voice-to-output run has not been verified. <a href="#evidence">Evidence and limits</a>.</p></div></section>'''
page, n = re.subn(r'<section class="overview".*?</section>', map_section, page, count=1, flags=re.S)
assert n == 1
page = page.replace('<div class="eyebrow">G1 · open architecture decision</div><h2>What evidence permits automatic learning?</h2>', '<h2>G1. What evidence permits automatic learning?</h2>')
page, n = re.subn(r'<p class="gap-intro">.*?</p>', '<p class="gap-intro">The endpoint can propose an effect, but no automatic writer applies it. Consider a request that proposes remembering “use Codex.” Dispatch succeeds, but Handy has not read the destination back. Should the next request receive that correction?</p>', page, count=1, flags=re.S)
assert n == 1
page = page.replace('<h3>After the possible “code desk” correction</h3>', '')
page = page.replace('href="../index.html">All alternatives', 'href="../round-3/index.html">Round 3 alternatives')
page = page.replace('<title>Handy architecture · One voice session</title>', '<title>Handy architecture · Combined revision</title>')
page = page.replace('Standalone explainer for design review', 'Combined revision for design review')
page = re.sub(r'class="pill[^"]*"', 'class="lifetime-label"', page)
page = page.replace('</footer>', '<p><a href="DECISIONS.md">Design decisions</a> · <a href="SKILL-PROPOSAL.md">Generic skill guidance draft</a></p></footer>')
# Apply the user's section-03 choice while preserving the lifetime explanation.
old_after = re.search(r'<section class="chapter wrap" id="after">.*?</section>', page, re.S).group(0)
next_recording = re.search(r'<div class="story"><aside class="story-side"><b>The next recording</b><p>(.*?)</p></aside><div class="story-main">(.*)</div></div></section>$', old_after, re.S)
assert next_recording is not None
choices = (ROOT / "section-03/index.html").read_text(encoding="utf-8")
selected = re.search(r'<section class="alternative" id="A">.*?<div class="example">(.*?)</div></section>', choices, re.S).group(1)
selected = re.sub(r'<header>.*?</header>', '', selected, count=1, flags=re.S)
for name in ("flow", "arrow", "node", "checks", "stop", "success", "subnote", "effect", "caption"):
    selected = re.sub(r'class="([^"]*)"', lambda match, name=name: 'class="' + ' '.join('reply-' + value if value == name else value for value in match[1].split()) + '"', selected)
selected = selected.replace('href="../index.html#G1"', 'href="#G1"')
after = '<section class="chapter wrap" id="after"><div class="chapter-head"><div class="index">AFTER / 03</div><h2>Follow the reply through Handy</h2><p>The model proposes text. Handy decides whether it can still send that text to the original input.</p></div>' + selected + '<p class="reply-sources"><a href="#S6">Reply and input checks</a> · <a href="#S8">Session lifetime</a> · <a href="#S9">Final destination check and output</a></p></section>'
lifetime = '<section class="chapter wrap" id="next-recording"><div class="chapter-head"><h2>What survives the next recording?</h2><p>' + next_recording[1] + '</p></div>' + next_recording[2] + '<p class="reply-history">When audio saving succeeds, Handy may save history before dispatching output. A history entry does not prove insertion, and history is not replayed as a model conversation. <a href="#S9">History and output order</a>.</p></section>'
page = page.replace(old_after, after + lifetime)
page = page.replace('<a href="#after">After</a>', '<a href="#after">After</a><a href="#next-recording">Next recording</a>')
css = '''
.reply-flow{display:grid;grid-template-columns:minmax(270px,1fr) 28px minmax(340px,1.3fr) 28px minmax(245px,.85fr);gap:12px;align-items:start}.reply-flow pre{background:#f1f5f8;color:var(--ink);border:1px solid var(--line);padding:16px;font-size:14px}.reply-arrow{color:var(--blue);font-size:26px;text-align:center;align-self:center}.reply-node{border:1px solid #c8d7e3;padding:18px;background:white}.reply-node h3{color:var(--deep)}.reply-checks{list-style:none;margin:0;padding:0}.reply-checks li{padding:12px 0;border-top:1px solid var(--line);font-size:14px}.reply-checks b{display:block;margin-bottom:4px}.reply-stop{margin-top:12px;border:1px solid #dfb990;background:#fff5e8;padding:12px;font-size:14px}.reply-success{background:var(--pale)}.reply-subnote{font-size:14px}.reply-effect{display:grid;grid-template-columns:200px 1fr;gap:22px;margin-top:20px;padding-top:18px;border-top:1px solid var(--line)}.reply-effect p{margin:0}.reply-effect>b{color:#8d4e0c}.reply-caption,.reply-sources{font-size:13px;color:var(--muted);margin:10px 0 0}.reply-history{margin:18px 0 0}.reply-sources{margin-top:16px}
@media(max-width:1100px){.reply-flow{grid-template-columns:1fr}.reply-arrow{transform:rotate(90deg)}.reply-effect{grid-template-columns:1fr;gap:8px}}
.lifetime-label{font-weight:700;color:var(--deep)}
/* Shared review direction: restrained headings, full-width text, square containers. */
.wrap,.top-inner{max-width:1488px;padding-left:24px;padding-right:24px}
.hero{padding-top:22px;padding-bottom:22px}.hero h1{font-size:28px;line-height:1.2;letter-spacing:-.015em;max-width:none;margin:0 0 10px}
h2{font-size:24px;line-height:1.25;letter-spacing:-.015em;max-width:none}h3{font-size:18px;line-height:1.3}.lede{font-size:17px;max-width:none;margin:0}
.overview{padding:24px 0}.overview-header,.chapter-head{display:block;margin-bottom:20px}.overview-header p,.chapter-head p,.gap-intro{max-width:none;width:100%;font-size:16px}.chapter-head h2{margin-bottom:10px}.chapter-head .index{margin:0 0 7px}
.chapter,.gap-section{padding-top:36px;padding-bottom:36px}.story{margin:26px 0;gap:28px;grid-template-columns:230px minmax(0,1fr)}.story-side{border-top-width:1px}.evidence{padding-top:36px;padding-bottom:40px}
.photo,.stage,.request,.overlay-row,.callout,.gate-list li,.outcome>div,.lifetime,.next-run,.gap-card,.missing div,.option,pre,code{border-radius:0}
.gap-card{margin-top:18px;padding:22px;border-left-width:1px}.missing{margin:0 0 22px}.option.rec{border-color:var(--blue)}.request-top{background:#e8f0f8;color:var(--ink);border-bottom:1px solid #bad0e2}.request-top h3,.request-top p{color:var(--ink)}
.message{grid-template-columns:170px minmax(0,1fr);gap:20px}.result{grid-template-columns:minmax(0,1fr) 300px}.result pre{background:#f0f5f8;color:#193147;border:1px solid #dce7ee}.result .stage{border-left-width:1px}.gap-intro{margin-bottom:18px}
.diagram-scroll{overflow-x:auto;background:white;border:1px solid var(--line);padding:12px}.diagram{display:block;width:100%;min-width:1000px;height:auto;font-family:"Segoe UI",sans-serif}.diagram .box{fill:white;stroke:#abc7dc}.diagram .box-gray{fill:#f1f4f7;stroke:#d6e0e8}.diagram .box-blue{fill:#e8f0f8;stroke:#abc7dc}.diagram .path,.diagram .connector{stroke:#1e5a8a;stroke-width:2;fill:none;stroke-dasharray:none}.diagram .path{marker-end:url(#arrow)}.diagram .label{font-size:14px;font-weight:700;fill:#192534}.diagram .micro{font-size:10px;font-weight:700;letter-spacing:.4px;fill:#1e5a8a}.diagram .startup-label{font-size:9px;letter-spacing:0}.diagram .small{font-size:12px;fill:#4d6071}.scroll-hint{display:none}
@media(max-width:1000px){.story{grid-template-columns:190px minmax(0,1fr)}.result{grid-template-columns:1fr}.scroll-hint{display:block;font-size:13px;color:var(--muted);margin:8px 0}}
@media(max-width:700px){.wrap,.top-inner{padding-left:18px;padding-right:18px}.hero{padding-top:20px;padding-bottom:20px}.hero h1{font-size:25px}h2{font-size:22px}.story{display:block}.message{display:block}.gap-card{padding:16px}.options{grid-template-columns:1fr}.missing{grid-template-columns:1fr}.missing span{display:block;text-align:center;transform:rotate(90deg)}.chapter,.gap-section{padding-top:28px;padding-bottom:28px}}
'''
page = page.replace('</style>', css + '</style>')
# The contents rail is outside the centered reading column.
page = page.replace('<div class="request">', '<div class="request" id="request-data">', 1)
page, n = re.subn(r'<header class="top">.*?</header>', '', page, count=1, flags=re.S)
assert n == 1
contents = '''<details class="contents" id="contents" open><summary aria-label="Toggle table of contents" title="Expand or collapse contents"><svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><rect x="2" y="3" width="16" height="14" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7 3V17M10 7H15M10 10H15M10 13H15" fill="none" stroke="currentColor" stroke-width="1.5"/></svg><span>Contents</span></summary><nav class="contents-list" aria-label="Table of contents"><a href="#overview">Overview</a><a href="#before">1. Profile and focused app</a><a href="#during">2. Speech and context</a><a class="subsection" href="#request-data">Request messages</a><a href="#after">3. Reply and output</a><a href="#next-recording">The next recording</a><a href="#G1">G1. Automatic learning</a><a href="#evidence">Evidence and limits</a><div class="contents-reference"><a href="../round-3/index.html">Round 3 alternatives</a><a href="DECISIONS.md">Design decisions</a></div></nav></details><div class="content-pane" id="content-pane" tabindex="0" role="region" aria-label="Architecture explanation">'''
shell_css = '''
html,body{height:100%;overflow:hidden}body{display:grid;grid-template-columns:200px minmax(0,1fr);height:100dvh}body:has(.contents:not([open])){grid-template-columns:48px minmax(0,1fr)}
.contents{height:100dvh;min-width:0;margin:0;padding:0;background:#f1f5f8;border-right:1px solid var(--line);overflow:hidden;z-index:2}.contents summary{height:58px;display:flex;align-items:center;gap:12px;padding:0 14px;cursor:pointer;list-style:none;color:var(--deep);font-size:14px;font-weight:700;border-bottom:1px solid var(--line);white-space:nowrap}.contents summary::-webkit-details-marker{display:none}.contents summary svg{flex:none}.contents summary:focus-visible{outline:2px solid var(--blue);outline-offset:-4px}.contents:not([open]) summary span{display:none}.contents summary:hover{background:#e2ecf3}
.contents-list{display:block;height:calc(100dvh - 58px);overflow-y:auto;overscroll-behavior:contain;scrollbar-gutter:stable;padding:12px 8px 20px;font-size:14px}.contents-list a{display:block;padding:9px 8px;margin:2px 0;line-height:1.4;text-decoration:none;overflow-wrap:anywhere}.contents-list a:hover,.contents-list a:focus-visible{background:#e2ecf3}.contents-list .subsection{padding-left:22px;font-size:13px}.contents-reference{border-top:1px solid var(--line);margin-top:14px;padding-top:12px;font-size:12px}
.content-pane{height:100dvh;min-width:0;overflow-y:auto;overflow-x:hidden;overscroll-behavior:contain;scrollbar-gutter:stable;scroll-behavior:smooth;container:reading / inline-size}.content-pane:focus-visible{outline:2px solid var(--blue);outline-offset:-3px}.content-pane main{min-width:0}.content-pane section{scroll-margin-top:20px}
@container reading (max-width:1100px){.reply-flow{grid-template-columns:1fr}.reply-arrow{transform:rotate(90deg)}.reply-effect{grid-template-columns:1fr;gap:8px}.story{grid-template-columns:190px minmax(0,1fr)}.result{grid-template-columns:1fr}.scroll-hint{display:block;font-size:13px;color:var(--muted);margin:8px 0}}
@container reading (max-width:1000px){.options,.missing{grid-template-columns:1fr}.missing span{display:block;text-align:center;transform:rotate(90deg)}}
@container reading (max-width:700px){.wrap{padding-left:18px;padding-right:18px}.story,.message,.image-pair,.session-stage,.result,.outcome,.lifetime,.evidence-grid,.sources{display:block}.story-side{margin-bottom:18px}.message{padding:18px}.message .role{margin-bottom:12px}.overlay-row{display:block}.overlay-row img{max-width:100%;margin-bottom:12px}.gate-list{display:block}.gate-list li{margin-bottom:10px}.lifetime>div{border-right:0;border-bottom:1px solid var(--line)}.image-pair>*,.session-stage>*,.result>*,.outcome>*,.sources>*{margin-bottom:14px}.hero h1{font-size:25px}h2{font-size:22px}.gap-card{padding:16px}}
@media(max-width:900px){body,body:has(.contents:not([open])){grid-template-columns:48px minmax(0,1fr)}.contents[open]{width:200px;box-shadow:5px 0 12px #19253418}.contents:not([open]){width:48px}}
@media(prefers-reduced-motion:reduce){.content-pane{scroll-behavior:auto}}
'''
page = page.replace('</style>', shell_css + '</style>')
page = page.replace('<body>', '<body>' + contents, 1)
page = page.replace('</body>', '</div><script>if(matchMedia("(max-width: 900px)").matches)document.getElementById("contents").open=false;</script></body>')
assert 'class="pill' not in page
assert 'After the possible “code desk” correction' not in page
(ROOT / "index.html").write_text(page, encoding="utf-8")
print(ROOT / "index.html")
