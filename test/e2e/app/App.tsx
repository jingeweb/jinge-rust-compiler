import { KEY_DATA, Transition, TransitionGroup, expose, onUnmount, vm } from 'jinge';

import { HmrWidget } from './HmrWidget';
export const state = vm({
  show: true,
  destroy: true,
  appear: true,
  groupVisible: true,
  items: [1, 2, 3],
  name: 'a',
  animate: false,
});
export const events: string[] = [];
export const nativeEvents: { id: string; type: string; property: string; trusted: boolean }[] = [];
export const unmounts: string[] = [];
for (const type of [
  'transitionrun',
  'transitionend',
  'transitioncancel',
  'animationstart',
  'animationend',
  'animationcancel',
])
  document.addEventListener(type, (event) => {
    const e = event as TransitionEvent;
    nativeEvents.push({
      id: (e.target as HTMLElement).id,
      type,
      property: e.propertyName ?? (event as AnimationEvent).animationName,
      trusted: e.isTrusted,
    });
  });
window.__E2E__ = { state, events, nativeEvents, unmounts };
function Content(props: { id: string }) {
  onUnmount(() => unmounts.push(props.id));
  return (
    <div id={props.id} className="box">
      {state.name}
    </div>
  );
}
export function App() {
  expose({ name: () => state.name });
  return (
    <>
      <h1>Transitions</h1>
      <HmrWidget />
      <button id="toggle" on:click={() => (state.show = !state.show)}>
        toggle
      </button>
      <button id="toggle-destroy" on:click={() => (state.destroy = !state.destroy)}>
        toggle destroy
      </button>
      <button id="rename" on:click={() => (state.name = 'b')}>
        rename
      </button>
      <button id="remove-item" on:click={() => state.items.splice(1, 1)}>
        remove
      </button>
      <button id="add-item" on:click={() => state.items.push(4)}>
        add
      </button>
      <button id="reverse" on:click={() => state.items.reverse()}>
        reverse
      </button>
      <button id="toggle-group" on:click={() => (state.groupVisible = !state.groupVisible)}>
        group
      </button>
      <button id="animate" on:click={() => (state.animate = true)}>
        animate
      </button>
      <div id="keyframe-box" className={state.animate ? 'box pulse' : 'box'}>
        {state.name}
      </div>
      <section id="retained">
        <Transition
          isEnter={state.show}
          enterClass="shown"
          leaveClass="hidden"
          enterActiveClass="active"
          on:beforeEnter={() => events.push('beforeEnter')}
          on:afterEnter={() => events.push('afterEnter')}
          on:beforeLeave={() => events.push('beforeLeave')}
          on:afterLeave={() => events.push('afterLeave')}
          on:enterCancelled={() => events.push('enterCancelled')}
          on:leaveCancelled={() => events.push('leaveCancelled')}
        >
          <Content id="retained-box" />
        </Transition>
      </section>
      <section id="destroyed">
        <Transition
          destroyAfterLeave
          isEnter={state.destroy}
          enterClass="shown"
          leaveClass="hidden"
          enterActiveClass="active"
          on:beforeEnter={() => events.push('destroyBeforeEnter')}
          on:afterEnter={() => events.push('destroyAfterEnter')}
          on:beforeLeave={() => events.push('destroyBeforeLeave')}
          on:afterLeave={() => events.push('destroyAfterLeave')}
        >
          <Content id="destroyed-box" />
        </Transition>
      </section>
      <section id="appear">
        <Transition
          appear
          isEnter={state.appear}
          enterClass="shown"
          leaveClass="hidden"
          enterActiveClass="appear-active"
          on:beforeEnter={() => events.push('appearBeforeEnter')}
          on:afterEnter={() => events.push('appearAfterEnter')}
        >
          <Content id="appear-box" />
        </Transition>
      </section>
      <section id="group">
        {state.groupVisible && (
          <TransitionGroup
            loop={state.items}
            key={KEY_DATA}
            enterClass="shown"
            leaveClass="hidden"
            enterActiveClass="active"
          >
            {(each) => (
              <div id={`item-${each.data}`} className="box" data-key={each.data}>
                {each.data}
              </div>
            )}
          </TransitionGroup>
        )}
      </section>
    </>
  );
}
