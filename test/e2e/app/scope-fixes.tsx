import { bootstrap, vm, type JNode, type WithChildren } from 'jinge';
const state = vm({ enabled: true, rows: [{ id: 1 }] });
function Config(props: { config: { enabled: boolean; test(item: { id: number }): boolean } }) {
  return (
    <p id="config">
      {props.config.enabled}:{props.config.test({ id: 1 })}
    </p>
  );
}
function Slot(input: WithChildren<(data: { label: string }) => JNode>) {
  return <>{input.children({ label: 'provided' })}</>;
}
function App() {
  return (
    <>
      <Config config={{ enabled: state.enabled, test: (item) => item.id === 1 }} />
      <p id="some">{state.rows.some((item) => item.id === 1)}</p>
      <Slot>
        {function render(_slot) {
          return (
            <p id="self">
              {arguments[0].label}:{render.name}
            </p>
          );
        }}
      </Slot>
      <button
        id="toggle"
        on:click={() => {
          state.enabled = false;
          state.rows = [{ id: 2 }];
        }}
      >
        update
      </button>
    </>
  );
}
bootstrap(App, document.body);
