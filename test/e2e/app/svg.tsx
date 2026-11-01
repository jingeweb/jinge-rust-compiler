import { bootstrap, vm, type JNode, type WithChildren } from 'jinge';
function Dot() {
  return <circle id="component" cx="30" cy="30" r="20" fill="red" />;
}
function Frame(props: WithChildren<JNode>) {
  return (
    <svg width="120" height="60">
      {props.children}
    </svg>
  );
}
function Foreign() {
  return (
    <foreignObject width="100" height="50">
      <div id="html-child">HTML</div>
    </foreignObject>
  );
}
function App() {
  const state = vm({
    show: true,
    dots: [
      { id: 1, x: 30 },
      { id: 2, x: 80 },
    ],
  });
  return (
    <div>
      <h1>SVG namespaces</h1>
      <button id="toggle" on:click={() => (state.show = !state.show)}>
        toggle
      </button>
      <button id="reverse" on:click={() => state.dots.reverse()}>
        reverse
      </button>
      <svg width="120" height="60">
        <Dot />
      </svg>
      <svg width="120" height="60">
        {state.show && <circle id="conditional" cx="30" cy="30" r="20" />}
      </svg>
      <svg id="list" width="120" height="60">
        {state.dots.map((dot) => (
          <circle key={dot.id} data-id={dot.id} cx={dot.x} cy="30" r="20" />
        ))}
      </svg>
      <Frame>
        <circle id="slot" cx="30" cy="30" r="20" />
      </Frame>
      <svg width="120" height="60">
        <Foreign />
      </svg>
    </div>
  );
}
bootstrap(App, document.body);
