# 修复前决策记录

实际修复和验证结果见 [implementation.md](implementation.md)；以下保留修复前的决策过程。

日期：2026-10-31。依据用户对 README.md 审查项的回复及本次源码核对。本轮只记录决策，不修改实现；明确标为建议的内容不代表用户已经确认。

## 用户已明确的处理方式

- C02、C03：本轮不改，保留为待讨论项。作用域基础设施的修复不代表同时承诺解决方法依赖和回调依赖语义。
- C05：自动生成的表达式参数采用 `$jg$a0` 一类的低碰撞前缀。
- C10：对问题输入编译报错拒绝，spread 的后续能力设计保留待讨论。具体拒绝范围见下文。
- C10 范围已确认：不收窄当前正常工作的能力，保留单独组件 spread 的原对象透传；对 DOM spread、组件 spread 与新增 props/children/命名 slot 等会丢失语义的组合报错。ref 等独立特殊属性不能仅因与 spread 共存就拒绝，须按现有行为核实。
- C12：引入词法 scope/context 入栈出栈，完整处理符号表；map 转 For 的集合及模板 member expression 缺少 vm 包装时发出告警。实现须保留 SWC resolver 的绑定身份，避免重新按名字匹配。
- C12 告警范围补充：仅针对当前 TSX/JSX 文件定义的变量（包括顶层和组件内部），外部引入及其他无法确定的来源保持现状，不提示。本地变量若来自未知函数调用、context 或外部值，也不能仅凭声明位置判为非响应式。
- C13：`state[key]` 应监听 state 对应路径；普通 key 变量不是 const 绑定时告警，明确 key 自身赋值不会触发更新。响应式 key 路径仍保留动态监听。未来用特殊注释抑制告警的设计待讨论。
- C14：尽量提取 key；影响面可控时直接修复。
- C20：允许修改 hash，无业务兼容要求；按修改范围及性能选择方案。
- 未单独回复的问题：在影响面不扩散的前提下修复。
- 未支持语法：用户同意先明确拒绝提前 JSX return、map 解构/默认/rest 参数及第三个参数，并要求将不支持示例记入待讨论清单。

## 已确认的修复边界

1. **C10 的拒绝范围**：拒绝 DOM spread，以及组件 spread 与新增 props/默认 children/命名 slot 混用；保留单独 `<Child {...props} />` 的原对象透传。源码明确实现并注释了原对象透传能力，全部禁用 JSX spread 会收窄现有能力。ref 等编译器特殊属性按实际是否参与 props 合并分别核对。当前基础 demos 搜索未发现实际使用 JSX spread。
2. **C12 的告警范围**：本轮限当前文件中可确定为非响应式的变量；未知来源保持现状、不提示，不引入跨文件或 TypeScript 类型推断。

目前没有必须在实现前继续确认的事项；后续扩展方向保留在下方待讨论清单。

## 可直接实施的建议

- C09：优先在编译器内让常量 IDL 属性复用已有 property 赋值规则，与响应式路径保持一致；避免扩大通用运行时 setAttribute 的语义变化。
- C14：修正混合路径与字符串转义，提取块体最终 return 的 key；涉及局部计算或分支、不能安全外提的 key 明确诊断，避免生成越界变量引用。
- C20：优先使用独立、合法且唯一的内部函数编号；这同时覆盖默认 hash 和自定义 key，通常无需改变 hash 协议。
- C07：2026-11-01 追加决定并实现：SVG 专有标签直接按标签名选择 namespace，覆盖跨组件/插槽/If/For；foreignObject 子节点保持 HTML 边界。a/title/style/script 等同名标签暂不扩展跨组件推断。
- 将编译诊断纳入失败返回，替换已知输入错误的 panic/todo 分支；区分 warning 和 error，避免把现有“警告”文案全部升级为阻断错误。
- 测试先建立 Rust 转换/诊断、当前源码 binding + 真实运行时联测、TS 国际化及 Vite 测试；再补少量浏览器 smoke。C02、C03 保留已知失败记录，不误列为已修复。

## 后续待讨论清单

- C02：方法接收者及方法体依赖规则。
- C03：嵌套回调的依赖边界与监听策略。
- C10：spread 的支持范围、props 身份和 slots 合并规则。
- C13：特殊注释等告警抑制机制。
- C12：未来结合 TypeScript 的函数返回类型推断响应式来源；需讨论类型信息能表达的响应式契约及推断范围。
- 源码语言范围：未来可能完全放弃 JavaScript/JSX 源码支持，仅支持 TypeScript/TSX。当前不据此删除能力或新增语言限制。
- 未支持语法的后续扩展，以下输入本轮先报编译错误，不静默丢弃语义：

```tsx
// 组件提前 JSX return；当前只处理单个最终模板 return。
function A(props) {
  if (props.ok) return <p>yes</p>;
  return <p>no</p>;
}

// map 解构参数。
state.items.map(({ name }) => <p>{name}</p>);
state.items.map(([first]) => <p>{first}</p>);

// map 默认参数。
state.items.map((item = fallback) => <p>{item.name}</p>);

// map rest 参数。
state.items.map((...args) => <p>{args[0].name}</p>);

// map 第三个参数（原数组）。
state.items.map((item, index, array) => <p>{array.length}</p>);
```

上述限制只针对转换为模板/For 的语法，不应禁止普通业务 JavaScript 中的 map 或提前 return。
