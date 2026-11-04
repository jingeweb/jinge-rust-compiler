import { bootstrap, vm } from 'jinge';
const state = vm<{ multi: boolean; text: string; index: number; user: { name: string } | null }>({
  multi: true,
  text: 'jinge',
  index: 0,
  user: null,
});
function serviceLabel(this: { name: string }) {
  return this.name;
}
const methods = vm({ service: { name: 'Alice', label: serviceLabel } });
function App() {
  return (
    <>
      <h1>DOM and primitive paths</h1>
      <select id="constant" multiple>
        <option value="shanghai" selected>
          Shanghai
        </option>
        <option value="beijing" selected>
          Beijing
        </option>
      </select>
      <select id="reactive" multiple={state.multi}>
        <option value="shanghai" selected>
          Shanghai
        </option>
        <option value="beijing" selected>
          Beijing
        </option>
      </select>
      <select id="single" value="beijing">
        <option value="shanghai">Shanghai</option>
        <option value="beijing">Beijing</option>
      </select>
      <button id="toggle" on:click={() => (state.multi = !state.multi)}>
        mode
      </button>
      <input id="text" value={state.text} on:input={(event) => (state.text = event.target.value)} />
      <p id="trimmed">{state.text.trim()}</p>
      <p id="method-service">{methods.service.label()}</p>
      <button
        id="method-owner"
        on:click={() => {
          methods.service = { name: 'Bob', label: serviceLabel };
        }}
      >
        replace owner
      </button>
      <button
        id="method-function"
        on:click={() => {
          methods.service.label = function (this: { name: string }) {
            return this.name + '!';
          };
        }}
      >
        replace method
      </button>
      <p id="length">{state.text.length}</p>
      <p id="index">{state.text[state.index]}</p>
      <p id="optional">{state.user?.name.length}</p>
      <button id="index-next" on:click={() => (state.index = 1)}>
        index
      </button>
      <button id="user" on:click={() => (state.user = { name: '你好' })}>
        user
      </button>
    </>
  );
}
bootstrap(App, document.body);
