import { vm } from 'jinge';
const state = vm({ n: 0 });
export function HmrWidget() {
  return (
    <button id="hmr-counter" on:click={() => state.n++}>
      Original:{state.n}
    </button>
  );
}
