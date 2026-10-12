// Runs web/app.js against a fake DOM + fake Tauri invoke; drives actions; checks payloads.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert';

// usage: node tools/gui-test.mjs   (from the repo root)
const app = fs.readFileSync(new URL('../web/app.js', import.meta.url), 'utf8');
const m = (v) => ({ t: 'mouse', v });
const std = { left: m('1'), right: m('2'), wheel: m('3'), back: { t: 'key', v: 'F13', mods: [] }, forward: { t: 'key', v: 'F14', mods: [] }, up: m('9'), down: m('10'), dpi: { t: 'dpi' } };
const status = {
  name: 'DeathAdder V3 Pro', connection: 'Wireless dongle', firmware: '2.2', battery: 100, charging: true, polling: 1000,
  dpi: { active: 1, stages: [[1000, 1100], [800, 800], [1600, 1600], [3200, 3200], [6400, 6400]] },
  liftoff: { mode: 'asym', preset: 2, lift: 26, land: 25 },
  binds: { std, hs: { ...std, back: m('4'), forward: m('5') } },
};
let applied = null;
const root = { innerHTML: '' };
const listeners = {};
const ctx = {
  console, JSON, String, Number, Array, Object, Math, setTimeout,
  window: { __TAURI__: { core: { invoke: async (cmd, args) => {
    if (cmd === 'status') return JSON.parse(JSON.stringify(status));
    applied = args.changes;
    return { results: [{ section: 'DPI', ok: true }], status };
  } } } },
  document: {
    getElementById: (id) => (id === 'app' ? root : null),
    addEventListener: (t, f) => { listeners[t] = f; },
    activeElement: null,
    documentElement: {},
  },
  getComputedStyle: () => ({ getPropertyValue: () => '#6CC4FF' }),
};
vm.createContext(ctx);
vm.runInContext(app, ctx);
await new Promise((r) => setTimeout(r, 10));

const html = () => root.innerHTML;
const click = (act, arg) => listeners.click({ target: { closest: () => ({ dataset: { act, arg }, disabled: false }) } });
const change = (input, arg, value) => listeners.change({ target: { closest: () => ({ dataset: { in: input, arg }, value }) } });

assert.match(html(), /DeathAdder V3 Pro/);
assert.match(html(), /F13/);
assert.match(html(), /Settings are saved on the mouse/);
assert.doesNotMatch(html(), /pending/);

click('polling', '500');
assert.match(html(), /1 change pending: polling/);
click('revert');
assert.doesNotMatch(html(), /pending:/);

change('x', '0', '1234');
assert.match(html(), /steps of 50/);
assert.match(html(), /data-act="apply" disabled/);
change('x', '0', '1000');

click('sel', 'forward');
click('mod', 'Ctrl');
change('key', '', 'F15');
change('lift', '', '10'); // land (25) must drop below it
click('apply');
await new Promise((r) => setTimeout(r, 10));
const j = (o) => JSON.stringify(o); // payload comes from another vm realm
assert.strictEqual(j(applied.binds), j([{ button: 'forward', layer: 'std', action: { t: 'key', v: 'F15', mods: ['Ctrl'] } }]));
assert.strictEqual(j(applied.liftoff), j({ mode: 'asym', preset: 2, lift: 10, land: 9 }));
assert.strictEqual(applied.dpi, undefined);
assert.match(html(), /Written to the mouse and read back/);
console.log('gui harness ok');
