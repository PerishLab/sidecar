# 迁移到 Sidecar v0.8.1

v0.8.0 对使用者的全部要求依然成立，本次不新增任何要求。请读
`docs/CHANGELOG/v0.8.0/zh/MIGRATION.md`：戳丢掉了 `e` 字段、`inspect_socket` 变成不含地址的
inspect 段落、`[[inspect.endpoints]]` 整块移除，以及 `status`、`plan`、`targets.json` 的形状变更。
固定逃生口仍是 `sidecar reset --force`。

按精确版本安装过 v0.8.0 的人，拿到的产品与本次发布相同。跟随 `stable` 的人从 v0.7.0 直接到
v0.8.1，并且**仍然欠 v0.8.0 那份迁移** —— 因为 channel 从未指向过 v0.8.0。

`@perish/sidecar@0.8.0` 已发布，其绑定面与 `0.8.1` 相同；`latest` 标签随本次发布移动。
