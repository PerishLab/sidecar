# Sidecar v0.8.1

产品内容就是 v0.8.0 描述的那些。本次发布之所以存在，是因为 **v0.8.0 没能完成**：它的 lane 由一个
「只移动 manager、从不推进 channel 指针」的 Plumb 渲染，于是全部对象都发布了、运行也报告了 activation，
而 `stable` 仍然指着 v0.7.0。

Plumb v0.27.0 把这两件事分开，并渲染出同时执行两者的 lane。sidecar 的 lane 现在来自那个版本，
本次是第一个经由它们真正达成共识的 stable 发布。

v0.8.0 的对象仍然已发布且已封印。没有任何东西引用它们 —— 一个从未移动过的 channel 不会把任何人
指向那里；而 exact seal 是 create-only 的，所以它们原样保留。
