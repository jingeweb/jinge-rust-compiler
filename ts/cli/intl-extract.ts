import path from 'node:path';
import { statSync } from 'node:fs';
import { intlExtract } from '../intl/extract';

const langs = ['en', 'zhCn', 'zhTr'];

(async function () {
  const cwd = process.cwd();

  const srcDir = path.join(cwd, 'src');
  if (!statSync(srcDir).isDirectory()) {
    return;
  }
  const translateFilePath = path.resolve(cwd, './intl/translate.csv');
  const options = {
    srcDir,
    translateFilePath,
    languages: langs,
  };
  await intlExtract(options);
})().catch((ex) => {
  console.error(ex);
});
