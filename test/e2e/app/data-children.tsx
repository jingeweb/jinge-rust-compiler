import { bootstrap, type JNode, type WithChildren } from 'jinge';

type DataNode = { children: { label: string }[] };
function Forward(input: WithChildren<JNode>) {
  return input.children;
}
function App() {
  const config = { children: (node: DataNode) => node.children };
  const children = config.children({ children: [{ label: 'child' }] });
  return (
    <main>
      <span>{children[0].label}</span>
      <Forward>
        <strong>forwarded</strong>
      </Forward>
    </main>
  );
}
bootstrap(App, document.body);
