import libCoverage from 'istanbul-lib-coverage';
import libReport from 'istanbul-lib-report';
import reports from 'istanbul-reports';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import v8ToIstanbul from 'v8-to-istanbul';
const root = path.resolve(import.meta.dirname, '..');
const runtimeRoot = path.resolve(process.env.JINGE_RUNTIME_ROOT ?? path.join(root, '../jinge'));
const dir = path.join(root, 'coverage');
const map = libCoverage.createCoverageMap({});
const bundles = [
  ...readdirSync(path.join(root, 'dist'))
    .filter((f) => f.endsWith('.js'))
    .map((f) => path.join(root, 'dist', f)),
  ...readdirSync(path.join(root, 'dist/cli'))
    .filter((f) => f.endsWith('.js'))
    .map((f) => path.join(root, 'dist/cli', f)),
];
for (const name of bundles) {
  const converter = v8ToIstanbul(name, 0);
  await converter.load();
  for (const [file, coverage] of Object.entries(converter.toIstanbul())) {
    if (!file.startsWith(path.join(root, 'ts'))) continue;
    // A source-mapped bundle can contain many original files. Explicitly
    // zero every file; a synthetic V8 root range only remaps to one source.
    for (const key of Object.keys(coverage.s)) coverage.s[key] = 0;
    map.merge({ [file]: coverage });
  }
}
for (const file of readdirSync(path.join(dir, 'v8'))) {
  const data = JSON.parse(readFileSync(path.join(dir, 'v8', file), 'utf8'));
  for (const script of data.result) {
    if (!script.url.startsWith('file:')) continue;
    const name = fileURLToPath(script.url);
    if (
      !(
        name === path.join(runtimeRoot, 'dist/jinge.dev.js') ||
        name.startsWith(path.join(root, 'dist')) ||
        name === path.join(root, 'test/.compiled/tools.js')
      )
    )
      continue;
    const converter = v8ToIstanbul(name, 0);
    await converter.load();
    converter.applyCoverage(script.functions);
    for (const [file, coverage] of Object.entries(converter.toIstanbul())) {
      if (file.startsWith(path.join(root, 'ts')) || file.startsWith(path.join(runtimeRoot, 'src')))
        map.merge({ [file]: coverage });
    }
  }
}
for (const [name, predicate] of [
  ['compiler-ts', (file) => file.startsWith(path.join(root, 'ts'))],
  ['runtime', (file) => file.startsWith(path.join(runtimeRoot, 'src'))],
]) {
  const subset = libCoverage.createCoverageMap({});
  for (const file of map.files())
    if (predicate(file)) subset.addFileCoverage(map.fileCoverageFor(file));
  const context = libReport.createContext({ dir: path.join(dir, name), coverageMap: subset });
  for (const format of ['text', 'html', 'json-summary', 'lcovonly'])
    reports.create(format).execute(context);
}
