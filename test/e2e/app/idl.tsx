import { bootstrap, vm } from 'jinge';
const state = vm({ complete: 'username', enabled: true });
function App() {
  return (
    <>
      <input id="static-input" autoComplete="username" autoFocus readOnly />
      <video id="static-video" autoPlay />
      <input
        id="reactive-input"
        autoComplete={state.complete}
        autoFocus={state.enabled}
        readOnly={state.enabled}
      />
      <video id="reactive-video" autoPlay={state.enabled} />
      <button
        id="toggle"
        on:click={() => {
          state.complete = 'off';
          state.enabled = false;
        }}
      >
        toggle
      </button>
      <button
        id="clear"
        on:click={() => {
          state.complete = '';
          state.enabled = true;
        }}
      >
        clear
      </button>
    </>
  );
}
bootstrap(App, document.body);
