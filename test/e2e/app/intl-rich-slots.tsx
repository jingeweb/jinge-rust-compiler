import {
  type JNode,
  bootstrap,
  context,
  intlInit,
  intlSetLocale,
  onMount,
  onUnmount,
  t,
  vm,
} from 'jinge';

import { Chinese, English } from './intl-rich-slots-messages';

const state = vm({ name: 'Ada', clicks: 0, mounts: 0, unmounts: 0, open: true });
function Action(props: { children?: JNode }) {
  const model = context<typeof state>('rich-parent');
  onMount(() => {
    model.mounts++;
  });
  onUnmount(() => {
    model.unmounts++;
  });
  return (
    <button
      className="rich-action"
      on:click={() => {
        model.clicks++;
      }}
    >
      {props.children}
    </button>
  );
}
await intlInit({
  defaultLocale: 'rich-en',
  dicts: { 'rich-en': { message: English }, 'rich-zh': { message: Chinese } },
});
function App() {
  context('rich-parent', state);
  const prefix = 'prefix:';
  return (
    <section>
      <button id="zh" on:click={() => intlSetLocale('rich-zh')}>
        中文
      </button>
      <button
        id="toggle"
        on:click={() => {
          state.open = !state.open;
        }}
      >
        Toggle
      </button>
      <input
        id="name"
        value={state.name}
        on:input={(event) => {
          state.name = event.target.value;
        }}
      />
      <p id="rich">
        {state.open &&
          t(
            'Hello <B>{name}</B>',
            {
              name: state.name,
              B: (child: JNode) => (
                <Action>
                  {prefix}
                  {child}:{state.name}
                </Action>
              ),
            },
            { key: 'message' },
          )}
      </p>
      <span id="clicks">{state.clicks}</span>
      <span id="mounts">{state.mounts}</span>
      <span id="unmounts">{state.unmounts}</span>
    </section>
  );
}
bootstrap(App, document.body);
