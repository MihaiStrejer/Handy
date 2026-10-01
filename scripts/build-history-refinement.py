"""Build the selected History inspector refinement from the round-one assets."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
source = (root / 'design/directions/round-1.html').read_text(encoding='utf-8')
source = source.replace('History processing details</title>', 'History inspector and failure states</title>')
source = source.replace('History, with the processing details', 'History inspector: status and failures')
source = source.replace('Three ways to inspect a run inside Handy’s existing design.', 'Option B selected. Status beside the timestamp; failure reasons where you can see them.')
source = source.replace('Design round 1', 'Design round 2')
source = re.sub(r'<nav class="jumps".*?</nav>', '<nav class="jumps" aria-label="Samples"><a href="#B">Interactive inspector</a><a href="#failure-samples">Failure examples</a></nav>', source)
source = re.sub(r'<footer class="board-footer">.*?</footer>', '<section id="failure-samples" class="board-footer"><h2>Failure examples</h2><p class="muted">A short reason stays with the recording. Open an example to see its explanation in the inspector.</p><div id="failures" class="failure-grid"></div></section><footer class="board-footer"><p><a href="../components/history-post-processing.md">Read the updated specification</a> · <a href="round-1.html">Previous layout comparison</a></p></footer>', source)
css = '''
.entry-top{display:flex;align-items:center;gap:10px;flex-wrap:wrap;justify-content:flex-start}
.entry-time{white-space:nowrap;font-weight:600}
.entry-top .status{margin:0;font-weight:400;white-space:nowrap;font-size:11px;display:inline-flex;gap:4px}
.entry-top .status .icon{width:13px;height:13px}
.entry-tools{margin-left:auto}
.entry-reason{font-size:12px;color:var(--error);margin:0 0 10px;overflow-wrap:anywhere}
.processing-summary .metadata{margin-bottom:5px}
.processing-summary{margin-bottom:10px}
.processing-summary .entry-reason{margin:5px 0}
.failure-explanation{padding:0 0 14px;margin:0 0 10px;border-bottom:1px solid var(--line)}
.failure-explanation h3{color:var(--error);font-size:14px;margin:0 0 7px}
.failure-explanation p{font-size:12px;margin:6px 0}
.failure-explanation .error-code{font-size:11px;color:var(--muted);margin-top:12px}
.failure-grid{display:grid;grid-template-columns:1fr 1fr;gap:20px;margin-top:20px}
.failure-grid .entry{margin:0;min-width:0}
.failure-grid .entry-tools{display:none}
.sample-actions{display:flex;gap:10px;align-items:center;margin-top:12px}
.failure-grid .sample-title{font-size:12px;color:var(--muted);margin:0 0 12px}
.entry-top .status.error{color:var(--error)}
@media(max-width:800px){.failure-grid{grid-template-columns:1fr}}
'''
source = source.replace('</style>', css + '</style>')
source = source.replace('<symbol id="history"', '<symbol id="warning" viewBox="0 0 24 24"><path d="m12 3 10 18H2ZM12 9v5M12 17h.01"/></symbol>\n<symbol id="history"')
source = re.sub(r'const options = \[.*?\];', "const options = [{id:'B',name:'Side inspector',note:'Status follows the time in the recording header.',mode:'inspector',risk:'The inspector fills the History pane in compact windows. Close it to return to the recording list.'}];", source, flags=re.S)
source = source.replace("open:false,tab:'overview'", "open:true,tab:'overview'")
failure_data = '''
const failures = {
 rate_limit:{name:'Rate limit',short:'The provider rate limit was reached.',title:'Provider rate limit reached',reason:'OpenAI rejected this request because its rate limit was reached.',next:'Wait before trying again. If this keeps happening, check your provider limits.',code:'HTTP 429 · rate_limit_exceeded',stage:'Rewrite request',time:'0.8 s',sent:true,usage:false},
 auth:{name:'API key rejected',short:'The provider rejected the API key.',title:'API key rejected',reason:'The provider could not authenticate this request.',next:'Check the API key in Post Process settings before trying again.',code:'HTTP 401 · authentication_failed',stage:'Rewrite request',time:'0.3 s',sent:true,usage:false},
 timeout:{name:'Request timed out',short:'No complete response arrived within 60 seconds.',title:'Request timed out',reason:'Handy stopped waiting after 60 seconds without a complete response.',next:'Check your connection and provider status. The provider may still have processed the request.',code:'Client timeout · response not received',stage:'Rewrite request',time:'60.0 s',sent:true,usage:false},
 invalid:{name:'Invalid response',short:'The response did not contain a valid rewrite.',title:'Could not use the provider response',reason:'The provider returned a response, but it did not match the required text, operation and effect format.',next:'Check that the selected model supports the profile response format.',code:'HTTP 200 · response_validation_failed',stage:'Response validation',time:'1.8 s',sent:true,usage:true},
 setup:{name:'Model not selected',short:'Select a post-processing model to run this profile.',title:'Post-processing model not selected',reason:'This profile is ready, but no post-processing model has been selected.',next:'Choose a model in Post Process settings. No API request was sent.',code:'Local configuration · model_missing',stage:'Before request',time:'0.0 s',sent:false,usage:false}
};
'''
source = source.replace('const states =', failure_data + '\nconst states =')

def replace_function(name, body):
    global source
    source, count = re.subn(r'^function ' + name + r'\(.*$', lambda _: body, source, flags=re.M)
    assert count == 1, name

replace_function('status', '''function status(s){if(failures[s.status])return `<span class="status error">${icon('warning')}Post-process failed</span>`;if(s.status==='legacy')return '<span class="status">Older entry</span>';return `<span class="status">${icon('check')}Post-processed</span>`}''')
replace_function('metadata', '''function metadata(s){if(s.status==='legacy')return '<div class="metadata">Post-processing details were not recorded</div>';const f=failures[s.status];if(f)return `<div class="processing-summary" aria-label="Post-processing result"><div class="metadata"><span class="provider">OpenAI${s.status==='setup'?'':' / demo-fast'}</span><span>General</span><span>${f.time}</span>${f.usage?'<span>14,956 tokens</span><span>Est. $0.0013</span>':''}</div><p class="entry-reason">${f.short}</p><p class="note">${f.sent?(f.usage?'Response usage retained. No processed text inserted.':'Original transcript saved. Usage not reported.'):'No API request sent. No provider charge.'}</p></div>`;return '<div class="processing-summary" aria-label="Post-processing result"><div class="metadata"><span class="provider">OpenAI / demo-fast</span><span>General</span><span>2.4 s</span><span>15,028 tokens</span><span>Est. $0.0014</span></div></div>'}''')
replace_function('entry', '''function entry(o,s){return `<article class="entry"><div class="entry-top"><time class="entry-time" datetime="2026-09-27T12:34:00" title="September 27, 2026 at 12:34 PM">Sep 27 · 12:34 PM</time>${status(s)}${tools()}</div><p class="original">Ask codecks to check the failing test.</p>${player()}${metadata(s)}${button('open','Processing details','detail-trigger',`aria-expanded="${s.open}" aria-controls="detail-${o.id}"`)}</article>`}''')
original_stats = re.search(r'^function stats\(.*$', source, flags=re.M).group()
source = source.replace(original_stats, original_stats.replace('function stats(', 'function successStats('))
source = source.replace('function successStats(', '''function stats(s){const f=failures[s.status];if(!f)return successStats(s);const pairs=[['Provider','OpenAI'],['Requested model',s.status==='setup'?'Not selected':'demo-fast'],['Profile','General · revision 4'],['Failed stage',f.stage],['Post-processing',f.time],['Input tokens',f.usage?'14,860':f.sent?'Not reported':'0'],['Cached input',f.usage?'14,000':f.sent?'Not reported':'0'],['Output tokens',f.usage?'96':f.sent?'Not reported':'0'],['Estimated USD',f.usage?'0.001322':f.sent?'Unavailable':'0.00']];return `<dl class="stats">${pairs.map(([k,v])=>`<div><dt>${k}</dt><dd>${v}</dd></div>`).join('')}</dl>`}
function successStats(''')
original_overview = re.search(r'^function overview\(.*$', source, flags=re.M).group()
source = source.replace(original_overview, original_overview.replace('function overview(', 'function successOverview('))
source = source.replace('function successOverview(', '''function overview(s){const f=failures[s.status];if(!f)return successOverview(s);return `<div class="failure-explanation"><h3>${f.title}</h3><p>${f.reason}</p><p>${f.next}</p><p class="muted">Original transcript saved. No processed text was inserted.</p><p class="error-code">${f.code}</p></div>${stats(s)}<p class="note">${f.usage?'The response reported usage, even though validation failed. The estimate includes those tokens.':f.sent?'Usage and cost are unknown because no usage was returned.':'No API request was sent, so this run has no provider charge.'}</p><div class="output"><div class="output-head">Original ${button('copy-original',icon('copy')+' Copy original')}</div><p>Ask codecks to check the failing test.</p></div>`}
function successOverview(''')
replace_function('requests', '''function requests(o,s){const f=failures[s.status];if(s.status==='legacy'||(f&&!f.sent))return `<div class="empty"><strong>${f?'No request was sent':'Exact requests unavailable'}</strong><p class="muted">${f?'Choose a post-processing model before starting another run.':'No request snapshot was retained for this entry.'}</p></div>`;return `<label class="call-select">Call <select data-call aria-label="Request call"><option value="rewrite" ${s.call==='rewrite'?'selected':''}>${f?'1':'2'} · Rewrite · ${f?f.time:'1.8 s'}</option>${!f?`<option value="probe" ${s.call==='probe'?'selected':''}>1 · Compatibility check · 0.6 s</option>`:''}</select></label><p class="note">Saved request snapshot · POST api.openai.com/v1/chat/completions</p>${message('System message',s.call==='probe'?probeSystem:system,'copy-system')}${message('User message',s.call==='probe'?probeUser:user,'copy-user')}<p class="note">Synthetic messages and usage. Credentials and response error bodies are excluded.</p>`}''')
original_calls = re.search(r'^function calls\(.*$', source, flags=re.M).group()
source = source.replace(original_calls, original_calls.replace('function calls(', 'function successCalls('))
source = source.replace('function successCalls(', '''function calls(s){const f=failures[s.status];if(!f)return successCalls(s);if(!f.sent)return '<div class="empty"><strong>No provider calls</strong><p>Configuration validation stopped this run before sending a request.</p></div>';return `<div class="call"><div class="call-head"><strong>1 · Rewrite</strong><span>${f.time}</span></div><p class="error">${f.code}</p><p>${f.usage?'14,860 input / 96 output · Est. $0.001322':'Usage and cost not reported.'}</p><p>${f.reason}</p></div><p class="note">Compatibility was already verified; no check was sent for this run. No automatic retry occurred.</p>`}
function successCalls(''')
source = source.replace("[['success','Success'],['failed','Failed'],['legacy','Older entry']]", "[['success','Success'],...Object.entries(failures).map(([id,f])=>[id,f.name]),['legacy','Older entry']]")
# Selecting a sample starts on its explanation; tab changes remain available afterwards.
source = source.replace("s.status=event.target.value;s.call='rewrite';render(o)", "s.status=event.target.value;s.call='rewrite';s.tab='overview';s.open=true;render(o)")
gallery = '''
$('#failures').innerHTML=Object.entries(failures).map(([id,f])=>`<article class="entry"><p class="sample-title">${f.name}</p><div class="entry-top"><time class="entry-time">Sep 27 · 12:34 PM</time>${status({status:id})}</div><p class="original">Ask codecks to check the failing test.</p>${metadata({status:id})}<button class="detail-trigger" data-sample="${id}">Inspect ${f.name.toLowerCase()}</button></article>`).join('');
document.addEventListener('click',event=>{const sample=event.target.closest('[data-sample]');if(!sample)return;Object.assign(states.B,{status:sample.dataset.sample,open:true,tab:'overview',call:'rewrite'});render(options[0]);$('#B').scrollIntoView({behavior:'smooth'});$('#B [role=tab][aria-selected=true]').focus({preventScroll:true})});
'''
source = source.replace('let toastTimer;', gallery + '\nlet toastTimer;')
(root / 'design/directions/round-2.html').write_text(source, encoding='utf-8')
print('Wrote design/directions/round-2.html')
