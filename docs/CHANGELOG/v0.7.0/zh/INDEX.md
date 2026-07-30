# Sidecar v0.7.0

Sidecar 采用共享的 Plumb + Actions binary 闭包，并进入新的 stable 共识
路径。发布声明现在只存在于 `plumb.toml`。

发布版 `--version` 输出统一为 `sidecar vX.Y.Z`，从而让公共 manager
smoke 可以验证精确发布身份。channel 身份仍作为内部 update policy 输入。
