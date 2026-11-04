import { bootstrap, vm } from 'jinge';
const state = vm<{ inert: boolean | undefined }>({ inert: false });
function App() {
  return (
    <>
      <div id="bare" inert />
      <div id="yes" inert={true} />
      <div id="no" inert={false} />
      <div id="unset" inert={undefined} />
      <div id="reactive-inert" inert={state.inert}>
        <input id="inside" />
      </div>
      <button id="toggle-inert" on:click={() => (state.inert = !state.inert)}>
        toggle
      </button>
      <button id="unset-inert" on:click={() => (state.inert = undefined)}>
        unset
      </button>
    </>
  );
}
bootstrap(App, document.body);
