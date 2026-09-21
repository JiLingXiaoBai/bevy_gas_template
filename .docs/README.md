# bevy_gas_template 知识库

本工程是 GitHub 游戏仓库模板，默认以 `../bevy_gas` 的本地 path 依赖使用 GAS。配置数据、编译适配和开发工具均由本项目维护。
公共接口以当前代码 rustdoc 为准。

- [01 — 配置所有权、加载与扩展](01-configuration.md)
- [02 — Luban 配置工程与工具链](02-luban-toolchain.md)
- [03 — Excel 技能配置与 GAS 接入](03-gas-configuration.md)
- [04 — 模板使用与依赖维护](04-template-and-dependencies.md)
- [项目运行与导表命令](../README.md)
- [协作与编码规范](../AGENTS.md)

首次阅读顺序：`src/main.rs` → `src/gameplay.rs` → `src/config.rs`。
修改技能内容时维护本模板 `config/`；当前本地 GAS 已移除配置层。
首次创建游戏先阅读 04，准备与游戏同级的 GAS 源码，再按项目 README 准备工具、导表和运行。
