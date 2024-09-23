import path from 'node:path';
import { statSync } from 'node:fs';
import { intlCompile } from '../intl/compile';

(async function () {
  const cwd = process.cwd();

  const srcDir = path.join(cwd, 'src');
  if (!statSync(srcDir).isDirectory()) {
    return;
  }
  const translateFilePath = path.resolve(cwd, './intl/translate.csv');
  const outputDir = path.resolve(srcDir, './intl');
  const options = {
    srcDir,
    outputDir,
    translateFilePath,
    languages: ['zh-cn', 'zh-tr', 'en'],
  };
  await intlCompile(options);
  console.info('Intl Compile Done.');
})().catch((ex) => {
  console.error(ex);
});
