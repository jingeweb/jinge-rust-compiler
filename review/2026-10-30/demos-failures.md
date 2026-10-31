# jinge-demos 两个 e2e 失败的定位

日期：2026-10-31。对当前本地 binding 编译的最小 TSX 产物，在 Chrome 156.0.8078.73 中实际挂载验证；另做原生 DOM 对照。本次只定位，没有修改实现。机器可读结果见 demos-failures.json。

## D01：select multiple 初始化顺序错误

示例：`jinge-demos/packages/form/src/app.tsx` 的多选城市。模型和页面 JSON 汇总均包含上海、北京，控件只保留北京。

当前生成代码等价于：

```js
const select = createEleA('select', {}, selectedShanghaiOption, selectedBeijingOption);
select.multiple = true;
```

`createEleA` 已将两个 option 挂载。此时 select 仍处于默认的单选模式，第二个 selected option 会取消第一个的 selected 状态；之后改为多选不会恢复此前状态。并非模型数组丢值，也不是浏览器尚未刷新。

定位：`src/parser/jsx.rs` 第 44 行起，将常量 IDL 属性从 attrs 提取出来；第 78–90 行先创建带子节点的元素，再设置属性。运行时 `jinge/src/util/dom.ts` 的 `_createEl` 在创建函数返回前已 appendChildren。

历史对比确认：提交 8923bf7 的 C09 修复把常量 IDL 属性统一改到元素创建后赋值，**常量 multiple 的这处行为是此次修复引入的回归**。响应式属性原有路径同样在创建完子节点后首次赋值，因此响应式 `multiple={state.multiple}` 也有相同问题。

Chrome 验证：

| 场景                                    | 结果                      |
| --------------------------------------- | ------------------------- |
| 当前编译产物，常量 multiple             | multiple=true，只选中北京 |
| 当前编译产物，响应式 multiple 初值 true | multiple=true，只选中北京 |
| 原生 DOM，先 multiple=true，再挂载选项  | 上海、北京均选中          |
| 原生 DOM，先挂载选项，再 multiple=true  | 只选中北京                |

修复方向：对会影响子节点初始化行为的属性建立前置阶段，覆盖常量和响应式 multiple 的初次赋值。不能将全部 IDL 属性统一前移：原生对照中，在 option 挂载前设置 `select.value='b'`，最终会落到默认的 a；挂载后设置才正确为 b。

应补测试：常量 true/false、响应式初值 true/false、多选初始两项、单选/多选 value 和 selected，以及后续用户交互。支持运行时切换 multiple 时，不应凭空恢复模型以外的历史选择，需沿用浏览器/绑定规则。

## D02：路径读取不允许经过字符串

示例：`form.name.length`、`form.bio.length`。编译器生成：

```js
watchPathForRender(state, ['name', 'length'], render, host);
```

运行时 `jinge/src/vm/watch.ts:44` 的 `getValueByPath` 在中间值上执行：

```js
if (!isObject(val)) return undefined;
```

读到 `state.name` 后 val 是字符串，直接返回 undefined，尚未访问 length。空字符串也同样失败；数组是对象，因此数组 length 正常。这段限制由 git blame 确认来自旧实现，不是此次符号表或 e2e 变更引入。

真实编译产物验证：

| 表达式/操作                                | 结果                          |
| ------------------------------------------ | ----------------------------- |
| 初始 name='jinge' 的 length 渲染           | undefined，预期 5             |
| 初始 bio='' 的 length 渲染                 | undefined，预期 0             |
| items=[1,2] 的 length 渲染                 | 2                             |
| name 改为 'Alice!' 后，普通 JS 读取 length | 6                             |
| name 改变后模板 length 渲染                | 仍为 undefined                |
| 同时注册 watch(name) 与 watch(name.length) | 前者收到 Alice!，后者没有通知 |

更新也失败的原因：`handleVmChange` 会匹配到 ['name'] 是 ['name','length'] 的前缀，但重新取值仍返回 undefined；新旧值相等，普通 watcher 不通知。不能只修首屏文本赋值。

修复方向：审视中间值检查，允许 JavaScript 合法的字符串等原始值成员读取，保留 null/undefined 的安全终止，以及 ViewModel 根目标的既有约束。通过现有路径前缀通知即可响应 name 整体赋值，不需要为字符串创建 vm。

应补运行时路径取值和 watcher 测试，以及从 TSX 输入到 DOM 更新的联测：空/非空/中文或 emoji 字符串、替换字符串、数组 length、null/undefined 中间路径。字符串方法调用的 this 与依赖语义仍属于此前待讨论的 C02，不能把 length 修复等同于方法调用也已支持。
