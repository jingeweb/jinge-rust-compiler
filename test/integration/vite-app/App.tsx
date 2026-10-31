import { vm } from 'jinge';
export const state = vm({ n: 1 });
export function App() {
  return <button on:click={() => state.n++}>{state.n}</button>;
}
