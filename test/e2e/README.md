# Chrome 真实浏览器 e2e

```sh
pnpm test:chrome    # 先验证 Chrome 可启动并能读取 DOM
pnpm test:e2e
```

默认使用本机 Google Chrome，由 Playwright 启动独立 headless 实例，不使用日常浏览器的用户资料。可通过 `PLAYWRIGHT_CHANNEL` 选择已安装的其他 Chromium channel。本机实测 Chrome `156.0.8078.73`；CI 显式运行 `pnpm exec playwright install --with-deps chrome`。

测试前自动准备当前 Rust binding 和 TS 插件产物。需要同级运行时已构建，或设置 `JINGE_RUNTIME_ROOT`。Vite 服务器使用 `127.0.0.1:5197`，端口占用时明确失败，不复用其他服务。

12 个用例覆盖：

1. CSS transition 的真实 opacity/transform 中间值、可信 `transitionend`，保留 DOM 的响应式更新。
2. `destroyAfterLeave` 动画中保留节点、完成后仅销毁一次、再次进入时重建。
3. `appear` 初始动画。
4. leave→enter 取消及 `leaveCancelled`。
5. enter→leave 取消及 `enterCancelled`。
6. TransitionGroup 延迟移除、插入动画、key 重排的节点身份。
7. 父列表销毁时立即清理仍在离场动画中的节点。
8. 响应式 class 启动真实 CSS `@keyframes`、`CSSAnimation` 和可信 `animationend`。
9. 实际修改 TSX 文件，通过 Vite WebSocket 替换组件，确认没有整页刷新或重复事件处理。
10. TSX 编译失败显示真实 Vite overlay，修正后恢复，并检查没有 HMR 回调的二次异常。

11. 多选框常量/响应式 multiple 的初始选中项、模式切换及 select.value 初始化。
12. 原生输入事件驱动字符串长度、动态索引和可空路径更新。

动画断言没有手动 dispatch 结束事件，也没有冻结时间。页面初始样式先经历浏览器帧提交，再以 10ms 间隔在有界时间内采样中间值，并等待真实结束事件。首次采样调整后，相关用例连续 5 次通过；上轮原有 10 项连续两轮 **20/20 通过**，本轮新增回归后全部 12 项通过，没有重试放过失败。

HMR 测试串行修改本目录的 `app/HmrWidget.tsx`，在 finally 中恢复原内容。运行时请避免同时编辑这个测试专用文件。

结果：`coverage/e2e-report/index.html`、`coverage/e2e-results.json`。失败时另保留截图和 Playwright trace 到 `coverage/e2e-results/`。CI 上传这些产物；本机执行通过不代表远端 CI 已运行。

Transition 组件当前监听的是 CSS `transitionend`；独立的 `@keyframes` 用例验证的是编译产物保留浏览器原生动画能力，没有假定 Transition 会监听 `animationend`。

此次真实 overlay 测试额外复现并修复了一个局部问题：Vite 在模块编译失败时可能把 undefined 传给 accept 回调；插件现在先做空值保护，避免再次访问 `__hmrUpdate__` 抛错。
