import { bootstrap, vm, type WithChildren, type JNode } from 'jinge';
function Empty(props: Partial<WithChildren<JNode>>) {
  return <>{props.children && <b>provided</b>}</>;
}
function App() {
  const state = vm({ empty: true });
  return (
    <section>
      <button id="toggle" on:click={() => (state.empty = !state.empty)}>
        toggle
      </button>
      <main>{state.empty ? <Empty /> : <b>visible</b>}</main>
    </section>
  );
}
bootstrap(App, document.body);
