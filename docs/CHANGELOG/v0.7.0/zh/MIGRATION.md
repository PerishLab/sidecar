# 迁移到 Sidecar v0.7.0

解析 `sidecar --version` 的调用者不应再期待末尾的 channel 标注；精确
输出现在是 `sidecar vX.Y.Z`。

构建期 authority 字段改为 `SIDECAR_BUILD_AUTHORITY`。runtime state 与
`sidecar.toml` 均不需要迁移。
