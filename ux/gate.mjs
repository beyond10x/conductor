import fs from 'node:fs';
import {measureAll, flatten} from './measure.mjs';
import {compare, mainBaseline, worseThanMain, writeBaseline, readBaseline, baselinePath, hardFailures} from './ratchet.mjs';

const args = process.argv.slice(2);
const update = args.includes('--update');
const di = args.indexOf('--decision');
const decision = di >= 0 ? args[di + 1] : undefined;

const fi = args.indexOf('--from');
const metrics = fi >= 0 ? JSON.parse(fs.readFileSync(args[fi + 1], 'utf8')).gating_metrics : flatten(await measureAll({gatingOnly: true}));
const baseline = readBaseline();

if (update) {
  const open = hardFailures(metrics).filter((f) => !baseline?.scenarios?.[f.s]?.[f.k]?.decision && !(baseline?.scenarios?.[f.s]?.[f.k] && baseline.scenarios[f.s][f.k].value >= f.value));
  if (open.length && !decision) {
    console.error('hard-gate values above 0 enter or rise in the baseline only with --decision <id>:\n' + open.map((f) => `- ${f.s}/${f.k} = ${f.value}`).join('\n'));
    process.exit(1);
  }
  fs.writeFileSync(baselinePath, JSON.stringify(writeBaseline(metrics, baseline, decision), null, 2) + '\n');
  console.log('baseline written');
  process.exit(0);
}
if (!baseline) {
  console.error('no ux/baseline.json; run with --update');
  process.exit(1);
}
const main = mainBaseline();
if (!main) console.error('note: origin/main has no ux/baseline.json; the worse-than-main check is skipped');
const hard = hardFailures(metrics);
if (hard.length) console.error(`note: ${hard.length} hard-gate metrics are above 0 and held by the baseline (known failures):\n` + hard.map((f) => `- ${f.s}/${f.k} = ${f.value}`).join('\n'));
const problems = [...compare(baseline, metrics).problems, ...worseThanMain(baseline, main)];
if (problems.length) {
  console.error('ux gate failed:\n' + problems.map((p) => '- ' + p).join('\n'));
  process.exit(1);
}
console.log('ux gate ok');
