import path from 'node:path';
import { promises as fs } from 'node:fs';
import { intlExtract } from '../intl/extract';
import { loopMkdir, parseExtractArgvs } from '../intl/helper';

(async function () {
  const argv = parseExtractArgvs();
  {
    const csvDir = path.dirname(argv.translateCsv);
    await loopMkdir(csvDir);
    try {
      const st = await fs.stat(argv.translateCsv);
      if (!st.isFile()) {
        console.error(`${argv.translateCsv} is not file`);
        process.exit(-1);
      }
    } catch (ex) {
      if ((ex as { code: string }).code === 'ENOENT') {
        await fs.writeFile(argv.translateCsv, ''); // touch file
      } else {
        throw ex;
      }
    }
  }

  await intlExtract({
    srcDirs: argv.inputDirs,
    translateFilePath: argv.translateCsv,
    languages: argv.languages,
  });
})().catch((ex) => {
  console.error(ex);
});
