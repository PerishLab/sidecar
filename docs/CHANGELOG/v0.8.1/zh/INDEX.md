# Sidecar v0.8.1

产品内容就是 v0.8.0 描述的那些。本次发布之所以存在，是因为 **v0.8.0 没能完成**：它的 lane 由一个
「只移动 manager、从不推进 channel 指针」的 Plumb 渲染，于是全部对象都发布了、运行也报告了 activation，
而 `stable` 仍然指着 v0.7.0。

Plumb v0.27.0 把这两件事分开，并渲染出同时执行两者的 lane。sidecar 的 lane 现在来自那个版本，
本次是第一个经由它们真正达成共识的 stable 发布。

v0.8.0 的对象仍然已发布且已封印。没有任何东西引用它们 —— 一个从未移动过的 channel 不会把任何人
指向那里；而 exact seal 是 create-only 的，所以它们原样保留。

Sidecar 闭合了它存在的理由：声明一个目标、以干净的命令行拉起它、把授予的资源交到它手上、
知道它真的起来了、读回真实身份与日志、干净收场。

每个目标现在都由一个薄 host 父进程拉起，戳落在 host 上，因此**目标自己的命令行与 manifest
声明的逐字节相同**，任何第三方参数解析器都不会遇到 sidecar 的旗标。host 由五条律封闭：无策略、
无状态、除一次启动握手外无持久信道、无余寿、无变换。因为 host 经那次握手回报目标 pid，
`start` 会在命令不存在时失败，`status` 报的是目标自己的 pid 而不是父进程的。残差测试断言
「裸起与经 sidecar 起的差异恰好等于一份声明集」，现在在 Linux、macOS、Windows 三平台都跑。

`start --wait` 轮询 `health_url` 直到它答 2xx；没有配置 `health_url` 的目标会被点名拒绝，
而不是静默跳过。`logs` 读确定性路径，对从未启动过的目标同样有用。`status --format json`
带上日志路径。

租来的资源只计算一次、记录一次，并以三种形式呈现：manifest 值里的 `{term}` 模板、
`SIDECAR_<TERM>` 环境词、以及运行时状态里的记录。

**inspect 成为一项能力。** 目标只声明一个 inspect 段落，不写地址；sidecar 派生地址，
每个平台以一个 bridge 切面承载它 —— 有 Unix 域套接字的地方用套接字，Windows 用命名管道。
调用者拿到的是能力而非传输，因此承载它的东西可以在其之下更换。

`@perish/sidecar` 随本次发布首次发布。它把公告变成两个分面：`control` 的 `port()` 交出值，
`inspect` 的 `serve()` 永不交出地址。授予缺席时，对应分面也缺席。
