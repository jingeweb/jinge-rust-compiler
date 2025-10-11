import { loopMkdir, parseCompileArgvs } from '../intl/helper';

import { intlCompile } from '../intl/compile';

(async function () {
  const argv = parseCompileArgvs();
  await loopMkdir(argv.outputDir);
  await intlCompile({
    outputDir: argv.outputDir,
    translateCsvFile: argv.translateCsv,
  });
  console.info('Intl Compile Done.');
})().catch((ex) => {
  console.error(ex);
});
