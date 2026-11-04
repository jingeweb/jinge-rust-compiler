# TODO 细节备忘录

本文档用于补充 README.md 中 TODO 章节的细节备忘。

## typescript 类型推断增强

如果只支持 typescript，可以在单文件内做类型跟踪，跨文件一定程度上也能做。就可以做编译期的 computed 依赖追踪。比如：`<span>{someFn()}</span>`，someFn 中如果读取了 state.name，那就能建立起这个追踪，当 state.name 变化时触发 someFn 重新调用并更新视图。

同时，对于属性的vm包裹决策可以更准确。当前的逻辑，为了避免所有组件的属性值传递都vm包裹增加开销，当判定属性里没有可监听表达式时不包裹vm。比如：`<Child name={name} />` 这个组件，实际传递时，属性对象不用 vm 包裹，因为 name 变量就算变了，也无法监听到。`<Child name={state.name}>` 才会被 vm 包裹传递属性，并在 state.name 变更时通过属性传递到 Child 中。

但也就意味着，比如下面的例子，`props.config.disabled` 中，props 是非vm包装，也就是 member expr 的根 obj 不是vm包装时，哪怕 config 是 vm 包装，也无法在 disabled 变化时更新 Child 的视图。

```tsx
import { vm } from 'jinge';

function Child(props: { config: { disabled: boolean } }) {
  return <input placeholder="嵌套路径：不会更新" disabled={props.config.disabled} />;
}

function Parent(props: { disabled: boolean }) {
  // props 是 VM，但传给 Child 的 { config: props } 是普通对象。
  return <Child config={props} />;
}

export default function App() {
  const state = vm({ disabled: false });

  return (
    <div>
      <button on:click={() => (state.disabled = !state.disabled)}>
        切换禁用：{String(state.disabled)}
      </button>

      <p>
        <input placeholder="直接路径：正常更新" disabled={state.disabled} />
      </p>

      <Parent disabled={state.disabled} />
    </div>
  );
}
```

更简单的例子是：

```tsx
import { vm } from 'jinge';

export default function App() {
  const a = { b: vm({ x: 0 }) };

  return (
    <div>
      <button on:click={() => a.b.x++}>增加</button>
      <span>{a.b.x}</span>
    </div>
  );
}
```

引入 ts 类型系统后，就可以通过感知类型来决策，`<Child config={props} />` 中，props 是不是vm包裹的，传递给 Child 的 `{ config: props }` 是否需要 vm 包裹。

## 动态插槽（能力待讨论）

当前插槽按设计静态、一次性传递；`props.option['slot:label'] ?? props.option.label` 只在挂载时选择分支。之后新增、替换或移除插槽，不会重新选择或挂载对应模板；回退分支中的响应式文本仍可更新。

一个可能需要动态插槽的场景是按需加载业务扩展：通用选择器先显示负责人名称，人员模块加载后才提供带状态、部门的展示模板。变化的是渲染器本身；如果只是异步获取数据，预先传入静态插槽、在插槽内读取响应式数据即可。

下面是可直接作为 `App.tsx` 的最小演示。为保持单文件，展示模板定义在文件中，用定时器模拟模块加载；实际业务可换成动态 `import()`，从扩展模块取得模板。

```tsx
import { type JNode, vm } from 'jinge';

type Option = {
  label: string;
  'slot:label'?: JNode;
};

// 模拟由人员扩展模块提供的展示模板。
function EmployeeLabel() {
  return (
    <span>
      🟢 <strong>Ada</strong>
      <small> · 研发部</small>
    </span>
  );
}

function Selection(props: { option: Option }) {
  return (
    <div>
      当前负责人：
      {props.option['slot:label'] ?? props.option.label}
    </div>
  );
}

export default function App() {
  const state = vm<{ option: Option; loading: boolean }>({
    option: { label: '负责人 #42' },
    loading: false,
  });

  function loadEmployeeModule() {
    state.loading = true;
    setTimeout(() => {
      state.option = {
        label: 'Ada',
        'slot:label': EmployeeLabel,
      };
      state.loading = false;
    }, 600);
  }

  return (
    <div>
      <Selection option={state.option} />
      <button disabled={state.loading} on:click={loadEmployeeModule}>
        加载人员展示模块
      </button>
    </div>
  );
}
```

初始显示“当前负责人：负责人 #42”。点击按钮后，若支持动态插槽，应挂载 `EmployeeLabel`，显示状态、加粗姓名和部门；当前实现只把回退文本更新为“Ada”，不会挂载该模板。

若支持，需监听插槽存在性及函数引用变化，切换时卸载旧分支、挂载新分支，并正确处理插槽参数、订阅和生命周期清理。是否提供显式动态插槽能力、如何控制静态插槽的开销，后续再决策。
