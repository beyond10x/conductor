// The dashboard's UX gate, every merge (story:dashboard-ui-loop; `task ui:ux-gate`, part of
// `task check`): the contract's break-tests, then the layout contract, axe-core and the screen-only
// and keyboard-only users over every scenario state at the gating viewports in both themes, then the
// ratchet over ux/baseline.json. No key and no network: the page reads only the committed dist/ and
// its store fixture.
//
//   node ux/gate.ts                       gate; exit 1 on any problem
//   node ux/gate.ts --update [--decision <id>]   rewrite the baseline from this measurement
import { writeFileSync } from 'node:fs';
import { chromium } from 'playwright';
import { breakTests } from './breaks.ts';
import { flatten, measureAll } from './measure.ts';
import { baselinePath, compare, decided, hardFailures, mainProblems, needsDecision, readBaseline, readMain, undecided, worseThanMain, writeBaseline } from './ratchet.ts';

const args = process.argv.slice(2);
const update = args.includes('--update');
const at = args.indexOf('--decision');
const decision = at >= 0 ? args[at + 1] : undefined;
if (at >= 0 && (!decided(decision) || decision.startsWith('--'))) {
  console.error('ux gate failed: --decision names no id; an empty or blank one is no decision');
  process.exit(1);
}

const browser = await chromium.launch();
const breaks = await breakTests(browser);
await browser.close();
if (breaks.silent.length > 0) {
  console.error(`ux gate failed: the contract misreads ${breaks.silent.length} of ${breaks.ran} break-tests:\n${breaks.silent.map((name) => `- ${name}`).join('\n')}`);
  process.exit(1);
}
console.log(`break-tests: ${breaks.ran} of ${breaks.ran} read as built`);

const measured = await measureAll({ gatingOnly: true });
const metrics = flatten(measured);
const values = Object.values(metrics).reduce((sum, keys) => sum + Object.keys(keys).length, 0);
console.log(`measured ${measured.rows.length} state cases over ${measured.scenarios.length} scenarios, ${values} values`);
const baseline = readBaseline();

if (update) {
  const open = needsDecision(metrics, baseline, decision);
  if (open.length > 0) {
    console.error(`hard-gate values above 0 enter or rise in the baseline only with --decision <id>, an id the value was not held under before:\n${open.map(({ s, k, value }) => `- ${s}/${k} = ${value}`).join('\n')}`);
    process.exit(1);
  }
  writeFileSync(baselinePath, `${JSON.stringify(writeBaseline(metrics, baseline, decision), null, 2)}\n`);
  console.log(`baseline written: ${baselinePath}`);
  process.exit(0);
}
if (baseline === null) {
  console.error('ux gate failed: no ux/baseline.json; run task ui:ux-gate -- --update');
  process.exit(1);
}
const main = readMain();
const read = mainProblems(main, process.env.CI !== undefined && process.env.CI !== '' && process.env.CI !== 'false');
for (const note of read.notes) {
  console.error(`note: ${note}`);
}
const hard = hardFailures(metrics);
if (hard.length > 0) {
  console.error(`note: ${hard.length} hard-gate values are above 0 and held by the baseline with a decision:\n${hard.map(({ s, k, value }) => `- ${s}/${k} = ${value}`).join('\n')}`);
}
const problems = [...read.problems, ...undecided(baseline), ...compare(baseline, metrics).problems, ...worseThanMain(baseline, main.kind === 'baseline' ? main.baseline : null)];
if (problems.length > 0) {
  console.error(`ux gate failed:\n${problems.map((problem) => `- ${problem}`).join('\n')}`);
  process.exit(1);
}
console.log(`ux gate ok: ${values} values at the baseline`);
