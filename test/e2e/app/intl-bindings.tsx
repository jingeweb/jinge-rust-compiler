import { bootstrap, t as translate, vm, intlInit, intlSetLocale } from 'jinge';
await intlInit({
  defaultLocale: 'binding-zh',
  dicts: {
    'binding-zh': {
      NhX4DJ: '你好',
      greet: (params: Record<string, unknown>) => `你好 ${String(params.name)}`,
    },
    'binding-en': {
      NhX4DJ: 'Hello',
      greet: (params: Record<string, unknown>) => `Hello ${String(params.name)}`,
    },
  },
});
function t(text: string) {
  return '[local]' + text;
}
function App() {
  const state = vm({ name: 'Alice' });
  return (
    <section>
      <button id="en" on:click={() => intlSetLocale('binding-en')}>
        English
      </button>
      <button id="zh" on:click={() => intlSetLocale('binding-zh')}>
        中文
      </button>
      <input id="name" value={state.name} on:input={(event) => (state.name = event.target.value)} />
      <p id="translated" title={translate('Hello') + '!'}>
        {translate('Hello')}
      </p>
      <p id="params">{translate('Hello {name}', { name: state.name }, { key: 'greet' })}</p>
      <p id="local">{t('Hello')}</p>
    </section>
  );
}
bootstrap(App, document.body);
