# bevy_gas_template 知识库

本工程是 GitHub 游戏仓库模板，默认以 `../bevy_gas` 的本地 path 依赖使用 GAS。配置数据、编译适配、输入绑定、空间随机、逻辑位置、空间索敌、表现与开发工具均由本项目维护。
公共接口以当前代码 rustdoc 为准。

- [01 — 配置所有权、加载与扩展](01-configuration.md)
- [02 — Luban 配置工程与工具链](02-luban-toolchain.md)
- [03 — Excel 技能配置与 GAS 接入](03-gas-configuration.md)
- [04 — 模板使用与依赖维护](04-template-and-dependencies.md)
- [05 — 空间、索敌与 GAS 接入](05-spatial-targeting.md)
- [06 — 输入绑定与空间随机](06-input-and-spatial-random.md)
- [07 — 架构与插件组合](07-architecture-and-composition.md)
- [08 — 游戏命令与结果反馈](08-game-commands.md)
- [09 — 从模板开始新游戏](09-starting-a-game.md)
- [项目运行与导表命令](../README.md)
- [协作与编码规范](../AGENTS.md)

首次阅读顺序：`src/main.rs` → `src/game.rs` / `src/game/plugin.rs` → `src/application.rs` → 按需阅读 `src/gameplay.rs` / `src/config.rs`。配置演示独立位于 `examples/configured_game/`，可以整体删除。
修改技能内容时维护本模板 `config/`；GAS 保留技能/效果执行和目标结果契约；配置、输入绑定、空间随机、空间索敌与表现由本模板维护。
首次创建游戏先阅读 04 和 09，准备与游戏同级的 GAS 源码后即可运行基础启动器。配置演示和修改表格才需要按 README 准备 Luban 工具并导表。
