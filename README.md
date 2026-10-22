# bevy_gas_template

用于创建独立 Bevy 游戏的 GitHub 仓库模板。模板提供配置编译、技能索引、输入绑定、独立空间随机、逻辑位置、动态 AABB 索敌及 Luban 工具；游戏从 `src/game/` 开始编写业务。默认通过同级目录的本地 path 依赖使用 GAS：

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
游戏拥有配置、输入绑定、空间采样、索敌和位置表现；GAS 提供技能、效果、属性和任务执行。修改本地库后，下次构建即可使用，无需提交或推送。

详细接入和模块原理见 [模板库使用指南与 GAS 工作原理](.docs/10-template-usage-and-gas-internals.md)，包含配置加载、完整代码示例、tick 时间线、各 GAS 模块、输入索敌和常见玩法。

## 从模板创建游戏

维护者上传本工程到 GitHub 后，在仓库 **Settings** 中勾选 **Template repository**。使用者选择 **Use this template → Create a new repository**，克隆自己的仓库，并准备同级 `bevy_gas`。参见 [GitHub 模板设置](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-template-repository) 与 [从模板创建仓库](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-repository-from-a-template)。

新游戏独立维护，模板后续修改不会自动同步，GitHub 也不会替换 Rust 包名。可以先沿用 `bevy_gas_template`；改名时同步 Cargo package/default-run、源码、测试和保留的示例导入，以及文档和技能上下文。详见 [模板使用与依赖维护](.docs/04-template-and-dependencies.md)。

1. 确认 `../bevy_gas/Cargo.toml` 存在，安装 Rust 1.97.1+，项目使用 edition 2024 与 Bevy 0.20.0。
2. 运行 `cargo fetch --locked`，然后 `cargo run` 启动基础程序。
3. 可直接删除整个 `examples/`，再在 `src/game/plugin.rs` 的 `GamePlugin` 中注册自己的组件、资源和系统。
4. 按需添加配置加载、输入、索敌、相机与场景。`tests/` 可以独立删除，但建议保留基础回归；`src/application/` 和 `config/generated/` 属于基础代码，应保留。

具体扩展方式见 [09 — 从模板开始新游戏](.docs/09-starting-a-game.md)。

## 基础启动器

`src/main.rs` 选择 Bevy 运行环境，安装共享 `SimulationPlugin` 和空业务入口 `GamePlugin`。
它不加载配置、不创建演示实体或相机。默认 feature 开启窗口时，`cargo run` 打开空窗口，关闭窗口退出；未开启 `windowed` 时自动持续无窗口运行。基础启动无需先安装 Luban 或导出二进制配置。

```powershell
cargo run
cargo run -- --headless
cargo run --no-default-features -- --ticks 20
cargo run -- --headless --ticks 120 --tick-hz 60 --seed 42
```

| 参数 | 含义 |
| --- | --- |
| `--headless` | 使用无窗口 runner 持续运行；未启用 `windowed` 时自动选择此模式 |
| `--ticks N` | 仅用于无窗口模式；N 必须是正数，达到该 fixed tick 数后请求退出，供启动检查使用 |
| `--tick-hz HZ` | 设置固定模拟频率，必须是可转换为非零 timestep 的有限正数；默认 20 Hz |
| `--seed U64` | 设置 GAS 随机种子；省略时使用 `SimulationSettings` 的固定默认值 |

未指定 `--ticks` 的无窗口程序持续运行，可用 Ctrl+C 结束。退出请求在帧末处理，同一帧可能继续执行追赶 tick，因此 `--ticks` 不是精确离线步进或回放接口。`--config` 属于下面的配置演示，基础启动器不接收该参数。`--headless` 控制运行方式；要同时去掉窗口和渲染依赖，使用 `--no-default-features`，按需加回 `--features config-validation`。

## 配置技能演示

`examples/configured_game/` 提供可删除的 Laser 演示，窗口、无窗口验收与演示测试均在该目录中。
当前导表流程支持 **Windows x64**，需要 Git、PowerShell 7.2+、系统 .NET Runtime 8+ 和 PATH 中的 7-Zip；其他平台尚未提供对应的 Luban 安装与导表流程。

首次演示运行前，在仓库根目录执行：

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run --example configured_game
```

窗口中按 Space 释放配置技能。伤害、消耗、冷却和技能时间线来自本项目 Excel。
无窗口演示固定执行 13 个 tick 并验收结果：

```powershell
cargo run --example configured_game -- --headless
cargo run --no-default-features --features config-validation --example configured_game -- --headless
```

初始目标生命 500、施法者法力 100；5 级技能完成后目标生命 320、法力 80、活跃技能为 0。验收失败返回非零退出码。若 RUST_LOG 隐藏了信息日志，可先设置 `$env:RUST_LOG = "info"`。

演示默认从编译时工程根目录的 `assets/config/` 加载数据，不依赖启动时工作目录；部署后可指定目录：

```powershell
cargo run --example configured_game -- --config "./assets/config"
cargo run --example configured_game -- --headless --config "./assets/config"
```

默认 feature 为独立的 `windowed` 和 `config-validation`。演示未启用 `windowed` 时须显式传入 `--headless`；请求窗口会返回错误。关闭 `config-validation` 后仍有基础解码和编译检查，但不要求 manifest。

## 修改配置

1. 编辑 `config/tables/` 的 Excel；表登记、枚举与内联成本 Bean 位于 `config/defines/gas.xml`。
2. 运行 `pwsh -NoProfile -File config/export.ps1`。
3. 验证自己的游戏加载流程；保留演示时，可运行 `cargo run --example configured_game -- --headless` 或窗口演示。

导表在临时项目中生成、编译并校验配置，全部通过才发布到 `config/generated/` 和 `assets/config/`。暂存 Cargo.toml 将本地 GAS 路径转为绝对路径，仍使用同一份库源码；构建使用独立的 `target/config-export/`。脚本按存在性复制 examples/tests，删除这些目录不影响导表流程。

技能的外部资源消耗直接写在 ability 表的 `additional_costs` 有类型列表中，每项为 i32 资源 ID
resource 和固定整数 amount，例如 `5001,1;5002,3`。顺序与重复项保留，空列表表示无额外消耗。
资源 ID 允许完整 i32 范围，直接传给 GAS，由游戏 Provider 按整数 ID 判断有效性与支付，无需名称池映射。
配置现在为七张表，不再维护独立 ability_additional_cost 表。旧成本行 ID 列表按原顺序展开，旧资源名称按游戏映射转成整数，并完整重导；详见 [配置迁移](.docs/03-gas-configuration.md#配置迁移)。

生成代码与 `.bytes` 必须来自同一次导表；不要手动编辑生成 Rust 或 manifest。配置校验工具可独立使用：

```powershell
cargo run --bin gas-config -- inspect assets/config 1001 5
cargo run --bin gas-config -- validate-config assets/config
```

演示验收依赖初始技能的数值和时序；修改这些内容时，同步更新 `examples/configured_game/` 中的验收和测试。自己的游戏业务不依赖该固定结果。

## 可复用 Gameplay 能力

`application::SimulationPlugin` 安装 GAS、`GameplaySchedulePlugin`、固定 tick 和 GAS 随机种子，不加载配置或场景，也不安装索敌、命令或表现。需要技能命令时在游戏插件中添加 `AbilityCommandPlugin`，它会补充 `TargetingPlugin`。GAS 已由共享模拟安装，不要重复添加。

`gameplay::commands::GameCommandQueue<C>` 缓冲输入、AI、UI 等产生的自定义命令。技能命令适配器保持直接目标与索敌命令的提交顺序，通过 `AbilityCommandResult` 关联命令、索敌和执行编号。配置演示使用该队列，并读取终态反馈。

`gameplay::position`、`gameplay::spatial`、`gameplay::targeting` 分别维护逻辑位置与表现、内部动态 AABB 树、目标规则与请求队列。移动系统放在 `GameplaySet::Movement`；请求生产可使用 GAS `RequestProducers`。空间同步与索敌在 GAS resolver 前执行，查询读取逻辑位置点；单位体积碰撞和攻击命中规则由游戏实现。

`gameplay::ability_input::AbilityInputBindings<Action>` 将逻辑动作映射到 ASC 的技能 Handle，设备映射与输入缓冲由游戏负责。`gameplay::randoms::SpatialRandom` 按需显式初始化，与 GAS 概率使用独立随机状态，复用现有 `rand` 和 Bevy 官方形状采样能力。

以下示例不依赖导表产物，保留 examples 时可运行：

```powershell
cargo run --no-default-features --example logical_position_targeting
cargo run --no-default-features --example ability_input_bindings
cargo run --no-default-features --example multi_unit_commands
cargo run --no-default-features --example charged_ability
```

它们分别演示位置与插值、输入绑定、多单位移动/施法、释放时激活 GAS 的蓄力与取消。
接口与验证见 [空间索敌](.docs/05-spatial-targeting.md)、[输入与随机](.docs/06-input-and-spatial-random.md)、[插件组合](.docs/07-architecture-and-composition.md) 和 [游戏命令](.docs/08-game-commands.md)。

## 检查

基础程序和可复用模块：

```powershell
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
cargo clippy --no-default-features --all-targets -- -D warnings
cargo test --no-default-features
cargo build
cargo run -- --headless --ticks 3
cargo run --no-default-features -- --ticks 3
```

保留 examples 时，还需导出配置后执行演示验收与示例测试。`cargo test` 不代替显式运行示例测试：

```powershell
cargo test --all-features --examples
cargo test --no-default-features --examples
cargo run --example configured_game -- --headless
```

## 开发边界

- `src/main.rs`：基础启动器和运行参数；`src/game.rs`、`src/game/`：自己的游戏业务入口。
- `src/application.rs`、`src/application/`：共享模拟设置与装配、通用结果类型和默认配置路径。
- `src/gameplay.rs`、`src/gameplay/`：命令与反馈、输入、随机、调度、位置/表现、空间索引和索敌。
- `config/`、`src/config.rs`、`src/config/`：游戏配置、生成类型和适配层。
- `examples/configured_game/`：独立配置演示及其测试，可随整个 examples 删除；库不再导出 LaserReport/run_headless/run_windowed。
- 同级 `../bevy_gas`：GAS 执行、属性聚合、任务机制与目标结果契约。

新增表或公式由游戏维护 Schema 与编译映射。新的原生 AbilityTask 机制、ModifierOperation 仍需库支持；当前没有通用任务注册协议。模板更新和 GAS 改动分别由游戏维护，接口或依赖变化后完成导表与游戏回归。

项目自带六个 Luban 技能、Codex MCP 入口和 Zed 的 `config-validation` 分析设置；工具与缓存位于 `tools/luban/`。完整说明见 [.docs/README.md](.docs/README.md)、[配置所有权](.docs/01-configuration.md)、[Luban 工具链](.docs/02-luban-toolchain.md) 和 [Excel 配置规则](.docs/03-gas-configuration.md)。

配置工具和初始样例最初基于 `bevy_gas` 提交 `38147000190ef887db091ab2c9c39a558edd5abe` 适配；该哈希仅记录代码出处，不是当前 path 依赖的版本锁定。保留 [MIT 许可证](LICENSE)、[Luban 配置许可证](config/LICENSE.Luban) 与 [Luban 技能许可证](.agents/skills/LICENSE.Luban)。
