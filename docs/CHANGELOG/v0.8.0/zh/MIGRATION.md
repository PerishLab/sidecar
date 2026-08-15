# 迁移到 Sidecar v0.8.0

本次发布不带任何兼容垫片。以下每一项都是硬切换；从其中任何一项脱身的固定逃生口是
`sidecar reset --force`，然后照当前接口面重写 manifest。

## 由旧版 sidecar 起的目标

戳丢掉了 `e` 字段，因此**旧版 sidecar 起的目标不再被本版发现**。升级前用旧二进制停掉全部目标，
或者升级后先跑 `sidecar reset --force` 再重新启动。

原来从戳的 `e` 字段读 broker endpoint 的目标，改为从绑定读取。戳只为进程表标记一个进程，
不携带任何配置。「目标应接受并忽略未知 `--sidecar-stamp` 参数」那条条款作废 —— 该旗标现在
落在 host 上，永远不出现在目标自己的命令行里。

## Manifest

`inspect_socket = "unix:///absolute/path.sock"` 由一个不含地址的 inspect 段落取代：

```toml
[[sidecars]]
name = "api"
command = "cargo"
inspect = {}
```

sidecar 在 `<data_home>/projects/<namespace>/` 之下派生地址，并作为 `inspect` 授予公告出去。
原先可以在 `inspect_socket` 里展开的 `{project}` / `{namespace}` / `{name}` 模板随之消失。
仍然写着传输的 manifest 会被拒绝，而不是被翻译。

项目级 `[[inspect.endpoints]]` 整块移除。**从来没有任何代码拨过它** —— 它只被校验和打印。
直接删掉该块。

## 输出形状

`status --format json` 用 `pid` 报目标、`hosts` 报其父进程，取代原来单一的 `pids` 数组，
并新增 `logPath`。

`plan --format json` 用布尔字段 `inspect` 取代 `inspectSocket`，不再输出 `inspectEndpoints`。
它也不再把戳追加进目标参数，`stamp.endpoint` 一并消失。

`targets.json` 把 host 记为 `pid`、目标记为 `target`，并把 `port` 与 `inspectSocket` 并入
一个 `grants` 对象。它是运行时状态，因此跨越这项变更的受支持做法是 `sidecar reset`。

## 平台覆盖

残差证明现在三平台都跑。Windows 上探针报不出父进程与进程组，因此那里的声明集不含这两项 ——
这是同一断言的一份更窄的证明。

两条 Windows 限制是**写明而非隐藏**的。Windows 的 spawn 携带全部可继承句柄，因此用管道捕获
`sidecar start` 的调用者会阻塞到 broker 退出，需要改为重定向到文件。Windows 的 inspect 切面
无法遵守读写超时，会阻塞到对端应答。
