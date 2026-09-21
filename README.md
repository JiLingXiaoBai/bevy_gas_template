# bevy_gas_template

用于创建独立 Bevy 游戏的 GitHub 仓库模板。游戏拥有 Excel、Schema、生成代码、配置编译器、技能索引及 Luban 工具，默认通过同级目录的本地 path 依赖使用 GAS：

```toml
bevy_gas = { path = "../bevy_gas" }
```

模板或由模板创建的游戏与 `bevy_gas` 放在同一个父目录：

```text
workspace/
├── bevy_gas/
└── my_game/    # The template or any game created from it.
```

游戏目录可以叫 `bevy_gas_template` 或其他名称，只要仍与 `bevy_gas` 同级，默认相对路径就有效。
请另行取得兼容的 GAS 源码放到该位置，或按实际位置修改 path。Cargo 不会为 path 依赖自动下载库。
当前本地 GAS 已移除配置层；游戏使用自己的配置模块。修改本地库后，下次构建即可使用，不需要提交或推送。

## 从模板创建游戏

维护者上传本工程到 GitHub 后，在仓库 **Settings** 中勾选 **Template repository**。使用者在模板页选择 **Use this template → Create a new repository**，再克隆自己的新仓库，并另行准备同级的 `bevy_gas` 源码。参见 [GitHub 模板设置](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-template-repository) 与 [从模板创建仓库](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-repository-from-a-template)。

新游戏独立维护，模板后续修改不会自动同步。GitHub 创建仓库不会替换文件中的 Rust 包名；可以先沿用 `bevy_gas_template`。需要改名时，同步修改 Cargo 包名、默认程序名、源码和测试导入，以及文档与技能上下文；详见 [模板使用与依赖维护](.docs/04-template-and-dependencies.md)。

## 首次运行

当前导表工具链支持 **Windows x64**，需要支持 Rust edition 2024 的工具链、Git、PowerShell 7.2+、系统 .NET Runtime 8+ 和 PATH 中的 7-Zip。项目使用 Bevy 0.19.1。其他平台暂未提供对应的 Luban 安装与导表流程。

先确认 `../bevy_gas/Cargo.toml` 存在，再在新仓库根目录依次运行。首次需要网络下载固定工具与其他 Cargo 依赖，二进制配置资产由导表生成：

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run
```

窗口中按 Space 释放配置技能。伤害、消耗、冷却和技能时间线来自本项目 Excel。
无窗口验收使用相同的配置和场景：

```powershell
cargo run -- --headless
```

初始目标生命 500、施法者法力 100；5 级火球完成后目标生命 320、法力 80、活跃技能为 0。验收失败以非零退出码返回；若 RUST_LOG 隐藏了信息日志，可先设置 `$env:RUST_LOG = "info"`。
默认数据目录由编译时工程根目录定位到 `assets/config/`，不依赖启动命令的工作目录。部署后可通过 `--config` 指定实际配置目录，例如：

```powershell
cargo run -- --config "./assets/config"
cargo run -- --headless --config "./assets/config"
```

## 修改配置

1. 编辑本项目 `config/tables/` 的 Excel；表登记和枚举位于 `config/defines/gas.xml`。
2. 运行 `pwsh -NoProfile -File config/export.ps1`。
3. 运行 `cargo run -- --headless`，或启动窗口验证自己的改动。

导表在临时项目中生成、编译并校验本游戏配置，全部通过才发布到 `config/generated/` 和 `assets/config/`。暂存 Cargo.toml 将当前本地 GAS 路径转为绝对路径，仍使用同一份库源码，根清单保持 `../bevy_gas`；构建使用独立的 `target/config-export/`，避免影响游戏的编译产物。首次导表需要单独编译校验程序及依赖。

生成代码与 `.bytes` 必须来自同一次导表；不要手动编辑生成 Rust 或 manifest。默认运行时不要求 manifest；开启 `config-validation` 时会校验清单、摘要和 Schema。

```powershell
cargo run --features config-validation --bin gas-config -- inspect assets/config 1001 5
cargo run --features config-validation --bin gas-config -- validate-config assets/config
```

无窗口模式的预期数值是当前火球样例的验收条件。改变该技能的数值或时序时，同步更新样例验收与测试；普通窗口模式直接使用导出的配置。

## 检查

```powershell
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
cargo build
cargo run -- --headless
```

## 开发边界

- 游戏配置和适配层：`config/`、`src/config.rs`、`src/config/`。
- 游戏行为与入口：`src/gameplay.rs`、`src/gameplay/`、`src/main.rs`。
- GAS 执行、属性聚合和任务机制：同级 `../bevy_gas` 中的本地库。
- 模板更新与 GAS 改动分别由游戏维护；本地库改动参与下次构建，接口或依赖变化后完成导表和游戏回归验证。

新增表或公式由游戏维护 Schema 与编译映射。新的原生 AbilityTask 机制、ModifierOperation 仍需库支持；当前未引入通用任务注册协议。当前已直接使用本地库，联调与可选 Git 依赖的说明见 [模板使用与依赖维护](.docs/04-template-and-dependencies.md#本地联调)。

项目自带六个 Luban 技能、Codex MCP 入口和 Zed 的 `config-validation` 分析设置。工具与缓存位于本项目 `tools/luban/`。完整说明见 [.docs/README.md](.docs/README.md)、[配置所有权](.docs/01-configuration.md)、[Luban 工具链与 MCP](.docs/02-luban-toolchain.md) 和 [Excel 表与 GAS 编译规则](.docs/03-gas-configuration.md)。

配置工具和初始样例最初基于 `bevy_gas` 提交 `38147000190ef887db091ab2c9c39a558edd5abe` 适配；该哈希仅记录代码出处，不是当前 path 依赖的版本锁定。保留 [MIT 许可证](LICENSE)、[Luban 配置许可证](config/LICENSE.Luban) 与 [Luban 技能许可证](.agents/skills/LICENSE.Luban)。
