// fang GUI: read the mouse, edit, Apply (write + read-back in Rust), close.
// State lives in `state`; every change re-renders the whole (small) page.

const invoke = window.__TAURI__.core.invoke;

const MOUSE_OPTS = [
  ['1', 'Left click'], ['2', 'Right click'], ['3', 'Middle click'],
  ['4', 'Back'], ['5', 'Forward'], ['9', 'Wheel up'], ['10', 'Wheel down'],
];
const KEYS = [
  ...Array.from({ length: 12 }, (_, i) => 'F' + (i + 13)),
  ...Array.from({ length: 12 }, (_, i) => 'F' + (i + 1)),
  ...Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i)),
  ...Array.from({ length: 10 }, (_, i) => String(i)),
  ...Array.from({ length: 10 }, (_, i) => 'Num' + i),
  'NumLock', 'Space', 'Enter', 'Esc', 'Tab',
];
const MODS = ['Ctrl', 'Shift', 'Alt', 'Win'];
const TYPES = [['mouse', 'Mouse'], ['key', 'Key'], ['dpi', 'DPI cycle'], ['hs', 'Hypershift'], ['off', 'Off']];
// Chip y position on its side of the diagram, and the point on the mouse it points at.
const BTNS = [
  { id: 'left', name: 'Left click', side: 'L', y: 8, dx: 170, dy: 70 },
  { id: 'forward', name: 'Forward (side)', side: 'L', y: 102, dx: 135, dy: 125 },
  { id: 'back', name: 'Back (side)', side: 'L', y: 152, dx: 135, dy: 174 },
  { id: 'up', name: 'Wheel up', side: 'R', y: 8, dx: 209, dy: 47 },
  { id: 'wheel', name: 'Wheel click', side: 'R', y: 62, dx: 209, dy: 66 },
  { id: 'down', name: 'Wheel down', side: 'R', y: 116, dx: 209, dy: 86 },
  { id: 'right', name: 'Right click', side: 'R', y: 170, dx: 250, dy: 120 },
  { id: 'dpi', name: 'DPI (bottom)', side: 'R', y: 262, dx: 209, dy: 240 },
];
const MOUSE_SVG = `
<path d="M140.6 32.9 C150 26 176 16 190.6 13.5 C193 13.5 194.5 15 195.1 18 L222.7 18 C223 15 224 13.5 226 13.8 C244 16 268 25 280.2 40.4 C284 47 286 75 286.1 116.5 C286.4 150 287 180 286.9 200 C286.8 228 285 250 281.7 266 C276 286 262 300 240 309 C226 313 208 313.5 195 310 C172 304 155 297 148.8 288.2 C139 272 134.5 252 133.5 230 C132.8 205 133.8 160 134.6 116.5 C135.2 82 135.8 55 136.4 44 C137 38 138.5 34.5 140.6 32.9 Z" fill="#1A1E23" stroke="#4A525C" stroke-width="1.5"/>
<path d="M141.6 36 C143.5 70 146 120 148.5 150 C149 158 152 162 157 162 L265 162 C270 162 272.6 158 273 150 C275 118 277.5 75 279.5 42" fill="none" stroke="#3A414A" stroke-width="1.2"/>
<path d="M209.3 95.6 L209.3 162" stroke="#3A414A" stroke-width="1.2"/>
<rect x="195.1" y="18" width="27.6" height="75.4" rx="7" fill="#15181C" stroke="#3A414A"/>
<rect x="205" y="30.5" width="9" height="2.6" rx="1.3" style="fill: var(--ok)"/>
<rect x="198.1" y="41.1" width="22.4" height="50.8" rx="5" fill="#262B31" stroke="#5A636E"/>
<path d="M201 47 L217.6 47 M201 52 L217.6 52 M201 57 L217.6 57 M201 62 L217.6 62 M201 67 L217.6 67 M201 72 L217.6 72 M201 77 L217.6 77 M201 82 L217.6 82 M201 87 L217.6 87" stroke="#3A414A" stroke-width="1"/>
<rect x="131.5" y="103" width="8" height="45" rx="3" fill="#262B31" stroke="#5A636E"/>
<rect x="131.5" y="150.5" width="8" height="48" rx="3" fill="#262B31" stroke="#5A636E"/>`;
const ICON_LINK = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></svg>';
const ICON_X = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"/></svg>';

const state = { phase: 'loading', error: '', info: null, saved: null, cur: null, sel: 'back', layer: 'std', note: null };

const clone = (o) => JSON.parse(JSON.stringify(o));
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
const dpiValid = (v) => { const n = Number(v); return Number.isInteger(n) && n >= 100 && n <= 30000 && n % 50 === 0; };

function label(b) {
  if (!b) return '?';
  if (b.t === 'mouse') return (MOUSE_OPTS.find(([v]) => v === b.v) || [, 'Mouse ' + b.v])[1];
  if (b.t === 'key') return [...(b.mods || []), b.v].join('+');
  if (b.t === 'dpi') return 'DPI cycle';
  if (b.t === 'hs') return 'Hypershift';
  if (b.t === 'off') return 'Disabled';
  return 'Other (fn ' + b.func + ')';
}

/** Mouse status (from Rust) -> the editable shape the page works on. */
function fromStatus(s) {
  return {
    stages: s.dpi.stages.map(([x, y]) => ({ x: String(x), y: String(y), linked: x === y })),
    active: s.dpi.active - 1,
    polling: s.polling,
    lift: { mode: s.liftoff.mode, preset: s.liftoff.preset, lift: s.liftoff.lift, land: s.liftoff.land },
    binds: s.binds,
  };
}
const dpiOf = (c) => ({ active: c.active + 1, stages: c.stages.map((s) => [Number(s.x), Number(s.linked ? s.x : s.y)]) });

function pending() {
  const { cur, saved } = state;
  const p = {
    dpi: !same(dpiOf(cur), dpiOf(saved)),
    polling: cur.polling !== saved.polling,
    lift: !same(cur.lift, saved.lift),
    binds: [],
  };
  for (const layer of ['std', 'hs']) {
    for (const b of BTNS) {
      if (!same(cur.binds[layer][b.id], saved.binds[layer][b.id])) p.binds.push({ button: b.id, layer, action: cur.binds[layer][b.id] });
    }
  }
  return p;
}

async function load() {
  state.phase = 'loading';
  render();
  try {
    const s = await invoke('status');
    state.info = s;
    state.saved = fromStatus(s);
    state.cur = clone(state.saved);
    state.phase = 'ready';
    state.note = null;
  } catch (e) {
    state.phase = 'nomouse';
    state.error = String(e);
  }
  render();
}

async function apply() {
  const p = pending();
  const changes = { binds: p.binds };
  if (p.dpi) changes.dpi = dpiOf(state.cur);
  if (p.polling) changes.polling = state.cur.polling;
  if (p.lift) changes.liftoff = state.cur.lift;
  state.phase = 'applying';
  render();
  try {
    const r = await invoke('apply', { changes });
    const edits = state.cur;
    state.info = r.status;
    state.saved = fromStatus(r.status);
    state.cur = clone(state.saved);
    // Keep the user's edits for whatever didn't make it, so they can retry.
    const failed = r.results.filter((x) => !x.ok);
    for (const f of failed) {
      if (f.section === 'DPI') { state.cur.stages = edits.stages; state.cur.active = edits.active; }
      else if (f.section === 'Polling') state.cur.polling = edits.polling;
      else if (f.section === 'Lift-off') state.cur.lift = edits.lift;
      else state.cur.binds = edits.binds;
    }
    state.note = failed.length
      ? { kind: 'err', text: failed.map((f) => `${f.section}: ${f.error}`).join(' · ') }
      : { kind: 'ok', text: 'Written to the mouse and read back. fang can be closed.' };
  } catch (e) {
    state.note = { kind: 'err', text: String(e) };
  }
  state.phase = 'ready';
  render();
}

// ---- edits -----------------------------------------------------------------

function edit(fn) {
  fn(state.cur);
  state.note = null;
  render();
}

function effLayer() {
  const hsAvailable = Object.values(state.cur.binds.std).some((b) => b.t === 'hs');
  return hsAvailable ? state.layer : 'std';
}

const actions = {
  retry: () => load(),
  revert: () => { state.cur = clone(state.saved); state.note = null; render(); },
  apply: () => apply(),
  pickStage: (i) => edit((c) => { c.active = +i; }),
  toggleLink: (i) => edit((c) => { const s = c.stages[i]; s.linked = !s.linked; if (s.linked) s.y = s.x; }),
  removeStage: (i) => edit((c) => { c.stages.splice(+i, 1); if (c.active >= c.stages.length) c.active = c.stages.length - 1; }),
  addStage: () => edit((c) => { const l = c.stages[c.stages.length - 1]; c.stages.push({ ...l }); }),
  polling: (hz) => edit((c) => { c.polling = +hz; }),
  liftMode: (m) => edit((c) => { c.lift.mode = m; }),
  preset: (v) => edit((c) => { c.lift.preset = +v; }),
  layer: (l) => { state.layer = l; render(); },
  sel: (id) => { state.sel = id; render(); },
  type: (t) => edit((c) => {
    const L = c.binds[effLayer()];
    if (t === 'mouse') L[state.sel] = { t: 'mouse', v: '1' };
    else if (t === 'key') L[state.sel] = { t: 'key', v: 'F13', mods: [] };
    else L[state.sel] = { t };
    // A Hypershift key is the same button on both layers; the library writes both.
    if (t === 'hs') { c.binds.std[state.sel] = { t: 'hs' }; c.binds.hs[state.sel] = { t: 'hs' }; }
  }),
  mod: (m) => edit((c) => {
    const b = c.binds[effLayer()][state.sel];
    const ms = b.mods || [];
    b.mods = MODS.filter((x) => (x === m) !== ms.includes(x));
  }),
};

const inputs = {
  x: (i, v) => edit((c) => { c.stages[i].x = v; if (c.stages[i].linked) c.stages[i].y = v; }),
  y: (i, v) => edit((c) => { c.stages[i].y = v; }),
  lift: (_, v) => edit((c) => { c.lift.lift = +v; if (c.lift.land >= c.lift.lift) c.lift.land = c.lift.lift - 1; }),
  land: (_, v) => edit((c) => { c.lift.land = Math.min(25, +v); if (c.lift.land >= c.lift.lift) c.lift.lift = c.lift.land + 1; }),
  key: (_, v) => edit((c) => { c.binds[effLayer()][state.sel].v = v; }),
  mouse: (_, v) => edit((c) => { c.binds[effLayer()][state.sel].v = v; }),
};

document.addEventListener('click', (e) => {
  const el = e.target.closest('[data-act]');
  if (el && !el.disabled) actions[el.dataset.act](el.dataset.arg);
});
// Commit edits on `change` (blur/Enter/release), not every keystroke: a re-render mid-typing
// or mid-drag would replace the element under the user. While dragging, only the readout moves.
document.addEventListener('change', (e) => {
  const el = e.target.closest('[data-in]');
  if (el) inputs[el.dataset.in](el.dataset.arg, el.value);
});
document.addEventListener('input', (e) => {
  if (e.target.type === 'range') e.target.nextElementSibling.textContent = e.target.value;
});

// ---- render ----------------------------------------------------------------

const seg = (items, width) => `<div class="f-seg${width ? ' right' : ''}"${width ? ` style="width:${width}px"` : ''}>${items.join('')}</div>`;
const segBtn = (on, act, arg, text, disabled = false) =>
  `<button class="${on ? 'on' : ''}" data-act="${act}" data-arg="${esc(arg)}" ${disabled ? 'disabled' : ''}>${text}</button>`;
const dot = (on) => (on ? '<span class="f-dot" title="Pending"></span>' : '');

function render() {
  const root = document.getElementById('app');
  const focused = document.activeElement && document.activeElement.id;
  if (state.phase === 'loading' && !state.cur) root.innerHTML = headerHtml() + '<div class="empty"><p>Reading mouse…</p></div>';
  else if (state.phase === 'nomouse') root.innerHTML = headerHtml() + noMouseHtml();
  else root.innerHTML = headerHtml() + mainHtml() + footerHtml();
  if (focused) document.getElementById(focused)?.focus();
}

function headerHtml() {
  const i = state.info;
  const meta = i
    ? `<span>${esc(i.connection)}</span><span class="f-mono">FW ${esc(i.firmware)}</span>
       <span class="batt"><svg width="22" height="12" viewBox="0 0 22 12" aria-hidden="true"><rect x="0.5" y="0.5" width="18" height="11" rx="2.5" fill="none" stroke="currentColor"/><rect x="19.5" y="3.5" width="2" height="5" rx="1" fill="currentColor"/><rect x="2" y="2" width="${(15 * i.battery / 100).toFixed(1)}" height="8" rx="1.5" style="fill: var(--batt)"/></svg>
       <span class="f-mono">${i.battery}%</span>${i.charging ? '<span>charging</span>' : ''}</span>`
    : '';
  return `<header><div class="brand"><img src="wordmark.png" alt="fang">${i ? `<span class="dev">${esc(i.name)}</span>` : ''}</div><div class="meta">${meta}</div></header>`;
}

function noMouseHtml() {
  return `<div class="empty">
    <svg width="72" height="104" viewBox="0 0 72 104" fill="none" aria-hidden="true"><path d="M36 2 C16 2 8 16 8 38 C8 66 10 84 14 92 C18 102 54 102 58 92 C63 82 64 66 64 38 C64 16 56 2 36 2 Z" stroke="#4A525C" stroke-width="2" stroke-dasharray="5 5"/><path d="M36 2 L36 40" stroke="#4A525C" stroke-width="2"/><rect x="32" y="16" width="8" height="16" rx="4" stroke="#4A525C" stroke-width="2"/></svg>
    <h1>No mouse found</h1>
    <p>Plug in the dongle or the cable, and make sure the mouse is switched on. If Razer Synapse is running, close it: it holds the mouse while open.</p>
    <p class="f-mono" style="font-size:12px">${esc(state.error)}</p>
    <button class="f-btn primary" data-act="retry">Try again</button>
  </div>`;
}

function mainHtml() {
  const { cur } = state;
  const p = pending();
  const busy = state.phase === 'applying';

  const stages = cur.stages.map((s, i) => {
    const y = s.linked ? s.x : s.y;
    return `<div class="stage-row">
      <button class="f-radio ${cur.active === i ? 'on' : ''}" data-act="pickStage" data-arg="${i}" aria-label="Make stage ${i + 1} active">${i + 1}</button>
      <input id="x${i}" class="f-in f-mono ${dpiValid(s.x) ? '' : 'bad'}" type="number" min="100" max="30000" step="50" value="${esc(s.x)}" data-in="x" data-arg="${i}" aria-label="Stage ${i + 1} X DPI">
      <button class="f-ic ${s.linked ? 'on' : ''}" data-act="toggleLink" data-arg="${i}" aria-label="Link X and Y" title="Link X and Y">${ICON_LINK}</button>
      <input id="y${i}" class="f-in f-mono ${s.linked || dpiValid(s.y) ? '' : 'bad'}" type="number" min="100" max="30000" step="50" value="${esc(y)}" data-in="y" data-arg="${i}" ${s.linked ? 'disabled' : ''} aria-label="Stage ${i + 1} Y DPI">
      <button class="f-ic" data-act="removeStage" data-arg="${i}" ${cur.stages.length <= 1 ? 'disabled' : ''} aria-label="Remove stage">${ICON_X}</button>
    </div>`;
  }).join('');
  const badDpi = cur.stages.some((s) => !dpiValid(s.x) || (!s.linked && !dpiValid(s.y)));

  const dpiCard = `<section class="f-card" aria-label="DPI stages">
    <div class="card-head"><h2 class="f-h">DPI stages</h2>${dot(p.dpi)}<span class="f-hint right">100–30000, step 50</span></div>
    <div class="stage-head"><span>Active</span><span class="r">X</span><span></span><span class="r">Y</span><span></span></div>
    ${stages}
    <div style="display:flex;align-items:center;gap:12px;min-height:32px">
      <button class="f-mod" data-act="addStage" ${cur.stages.length >= 5 ? 'disabled' : ''}>+ Add stage</button>
      ${badDpi ? '<p class="f-err">Each value must be 100–30000 in steps of 50.</p>' : '<p class="f-hint">The DPI button cycles these stages.</p>'}
    </div>
  </section>`;

  const pollCard = `<section class="f-card" aria-label="Polling rate">
    <div class="card-head"><h2 class="f-h">Polling rate</h2>${dot(p.polling)}</div>
    ${seg([125, 500, 1000].map((hz) => segBtn(cur.polling === hz, 'polling', hz, `<span class="f-mono">${hz}</span> Hz`)))}
  </section>`;

  const L = cur.lift;
  const liftBody = L.mode === 'preset'
    ? seg([['Low', 1], ['Medium', 2], ['High', 3]].map(([l, v]) => segBtn(L.preset === v, 'preset', v, l)))
    : `<label class="lift-row"><span>Lift-off</span><input id="lift" class="f-range" type="range" min="2" max="26" step="1" value="${L.lift}" data-in="lift"><span class="f-mono">${L.lift}</span></label>
       <label class="lift-row"><span>Landing</span><input id="land" class="f-range" type="range" min="1" max="25" step="1" value="${L.land}" data-in="land"><span class="f-mono">${L.land}</span></label>
       <p class="f-hint">Landing always stays below lift-off.</p>`;
  const liftCard = `<section class="f-card grow" aria-label="Lift-off distance">
    <div class="card-head"><h2 class="f-h">Lift-off distance</h2>${dot(p.lift)}
      ${seg([segBtn(L.mode === 'preset', 'liftMode', 'preset', 'Preset'), segBtn(L.mode === 'asym', 'liftMode', 'asym', 'Asymmetric')], 220)}
    </div>
    ${liftBody}
  </section>`;

  return `<main><div class="col">${dpiCard}${pollCard}${liftCard}</div>${buttonsCard(p)}</main>`;
}

function buttonsCard(p) {
  const { cur, saved, sel } = state;
  const hsAvailable = Object.values(cur.binds.std).some((b) => b.t === 'hs');
  const layer = effLayer();
  const LB = cur.binds[layer];
  const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();

  const lines = BTNS.map((b) => {
    const on = sel === b.id;
    const lx = b.side === 'L' ? 128 : 292;
    return `<line x1="${lx}" y1="${b.y + 23}" x2="${b.dx}" y2="${b.dy}" stroke="${on ? accent : '#3A414A'}" stroke-width="1" ${b.id === 'dpi' ? 'stroke-dasharray="3 3"' : ''}/>
      <circle cx="${b.dx}" cy="${b.dy}" r="4" fill="${on ? accent : '#5A636E'}"/>`;
  }).join('');
  const chips = BTNS.map((b) => {
    const changed = !same(LB[b.id], saved.binds[layer][b.id]);
    return `<button class="f-chip ${sel === b.id ? 'sel' : ''} ${changed ? 'chg' : ''}" style="left:${b.side === 'L' ? 0 : 292}px;top:${b.y}px" data-act="sel" data-arg="${b.id}" aria-label="${esc(b.name + ': ' + label(LB[b.id]))}">
      <span class="f-chip-n">${b.name}</span><span class="f-chip-v">${esc(label(LB[b.id]))}</span></button>`;
  }).join('');

  const bind = LB[sel];
  let detail = '';
  if (bind.t === 'key') {
    detail = `<label>Key <select id="key" class="f-sel" data-in="key">${KEYS.map((k) => `<option ${k === bind.v ? 'selected' : ''}>${k}</option>`).join('')}${KEYS.includes(bind.v) ? '' : `<option selected>${esc(bind.v)}</option>`}</select></label>
      ${MODS.map((m) => `<button class="f-mod ${(bind.mods || []).includes(m) ? 'on' : ''}" data-act="mod" data-arg="${m}" aria-pressed="${(bind.mods || []).includes(m)}">${m}</button>`).join('')}`;
  } else if (bind.t === 'mouse') {
    detail = `<label>Acts as <select id="mouse" class="f-sel" data-in="mouse">${MOUSE_OPTS.map(([v, l]) => `<option value="${v}" ${v === bind.v ? 'selected' : ''}>${l}</option>`).join('')}</select></label>`;
  } else if (bind.t === 'dpi') detail = '<p class="f-hint">Cycles through the DPI stages.</p>';
  else if (bind.t === 'hs') detail = '<p class="f-hint">Hold this button to switch every other button to its Hypershift bind.</p>';
  else if (bind.t === 'off') detail = '<p class="f-hint">The button does nothing.</p>';
  else detail = '<p class="f-hint">Set by other software (macro, media key, …). Pick a type above to replace it.</p>';

  const selBtn = BTNS.find((b) => b.id === sel);
  return `<section class="f-card" style="gap:12px;min-height:0" aria-label="Buttons">
    <div class="card-head"><h2 class="f-h">Buttons</h2>${dot(p.binds.length > 0)}
      ${seg([segBtn(layer === 'std', 'layer', 'std', 'Standard'), segBtn(layer === 'hs', 'layer', 'hs', 'Hypershift', !hsAvailable)], 240)}
    </div>
    <div class="diagram"><svg width="420" height="330" viewBox="0 0 420 330" aria-hidden="true">${MOUSE_SVG}${lines}</svg>${chips}</div>
    <div class="editor">
      <div class="editor-title"><span>${selBtn.name}</span><span class="f-hint">${layer === 'std' ? 'Standard' : 'Hypershift'} layer</span></div>
      ${hsAvailable ? '' : '<p class="f-hint">Set any button to Hypershift to use the Hypershift layer. Its binds apply while that button is held.</p>'}
      ${seg(TYPES.map(([t, l]) => segBtn(bind.t === t, 'type', t, l)))}
      <div class="editor-detail">${detail}</div>
    </div>
  </section>`;
}

function footerHtml() {
  const p = pending();
  const changes = [p.dpi && 'DPI', p.polling && 'polling', p.lift && 'lift-off', p.binds.length && 'buttons'].filter(Boolean);
  const n = changes.length;
  const badDpi = state.cur.stages.some((s) => !dpiValid(s.x) || (!s.linked && !dpiValid(s.y)));
  const busy = state.phase === 'applying';
  let msg;
  if (state.note && state.note.kind === 'err') msg = `<span class="f-err">${esc(state.note.text)}</span>`;
  else if (n) msg = `<span class="f-dot"></span>${n} ${n === 1 ? 'change' : 'changes'} pending: ${changes.join(', ')}`;
  else if (state.note) msg = `<span class="f-ok">${esc(state.note.text)}</span>`;
  else msg = '<span class="f-hint">Settings are saved on the mouse. fang doesn\'t need to stay open.</span>';
  return `<footer><span class="msg">${msg}</span>
    <div class="actions">
      <button class="f-btn" data-act="revert" ${n && !busy ? '' : 'disabled'}>Revert</button>
      <button class="f-btn primary" data-act="apply" ${n && !badDpi && !busy ? '' : 'disabled'}>${busy ? 'Writing…' : 'Apply'}</button>
    </div></footer>`;
}

load();
