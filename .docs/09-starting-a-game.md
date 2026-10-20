# 09 — 从模板开始新游戏

## 启动自己的业务入口

准备与游戏目录同级的 `bevy_gas`，保留 Cargo.toml 的默认 path 依赖。在仓库根目录执行：

```powershell
cargo fetch --locked
cargo run
```

基础程序安装 Bevy 运行环境、`SimulationPlugin` 和 `GamePlugin`。默认开启 `windowed` 时得到一个空窗口，关闭窗口退出；程序没有加载表格、创建相机或生成单位。基础启动不需要 Luban 工具和 `assets/config/` 二进制资产。

共享模拟默认 20 Hz、固定随机种子。基础启动器支持：

```powershell
cargo run -- --headless
cargo run -- --headless --ticks 120 --tick-hz 60 --seed 42
cargo run --no-default-features -- --ticks 20
```

无窗口模式使用持续运行的 runner；只有显式指定正数 `--ticks N` 才在达到该 fixed tick 数后请求退出，适用于启动检查。Bevy 在帧末处理退出，同一帧可能仍有追赶 tick；它不保证严格执行 N tick，也不是离线步进或回放 API。未指定时可用 Ctrl+C 停止。未启用 `windowed` 时自动无窗口运行；启用窗口时，`--ticks` 必须与 `--headless` 一起使用。`--tick-hz` 必须可转换为非零 timestep，`--seed` 接受 u64。它们只配置 GAS 模拟时钟与随机流，游戏自用的 `SpatialRandom` 仍需单独设置。

## 删除演示

1. 不需要示例时，直接删除整个 `examples/`。无需修改 Cargo.toml：示例由 Cargo 自动发现，库、主程序和基础测试不引用该目录。
2. 保留 `src/main.rs`、`src/game.rs`、`src/game/`，从 `GamePlugin` 开始开发自己的游戏。
3. 保留 `src/application/` 的共享模拟，以及 `src/config.rs` 引用的 `config/generated/`。生成 Rust 类型属于配置编译代码；删除演示不会解除这个依赖。
4. `tests/` 可独立删除，但建议保留配置、命令、索敌、位置和重放回归。Laser 专用测试位于示例目录，随示例一起删除。
5. 删除示例后跳过文档中的 `--example ...` 和 `cargo test --examples` 命令，继续运行基础 fmt、Clippy、test、build 和有限 tick 启动检查。

删除 examples 会移除全部演示 Rust 代码及专用测试。`config/tables/` 的起始 Excel、现有 Schema 和
`assets/config/` 导出数据仍保留为可替换的游戏配置；基础主程序不依赖这些运行时内容。
这次分离不重置表格或 Schema，需要更换玩法数据时按正常配置流程修改和导表。

无需为删除演示额外拆 crate、修改 GAS 依赖或重新导表。导表脚本按存在性复制 examples/tests，缺少这些目录仍可构建 `gas-config`。

## 在 GamePlugin 中添加玩法

`src/game.rs` 是业务门面，`src/game/plugin.rs` 实现空的 `GamePlugin`，主程序已经注册它。先在此处添加资源、系统和自己的领域插件。例如，用一个游戏 Resource 保存回合状态，并在 Startup 中生成逻辑实体：

```rust
use bevy::prelude::*;

/// Composes this game's state and systems after the shared simulation is installed.
pub struct GamePlugin;

#[derive(Resource, Default)]
struct MatchState {
    running: bool,
}

#[derive(Component)]
struct Player;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchState>()
            .add_systems(Startup, initialize_match);
    }
}

fn initialize_match(mut commands: Commands, mut state: ResMut<MatchState>) {
    commands.spawn(Player);
    state.running = true;
}
```

示意中的资源和系统应按规模移入 `src/game/` 下所属业务模块；新功能模块使用门面文件与同名目录。窗口表现代码按 `windowed` feature 编译，游戏自行决定相机、2D/3D 场景、UI 和输入方式。

主程序已安装 `SimulationPlugin`，它已包含 GAS 和游戏调度，不要在 `GamePlugin` 再次安装 GAS。按游戏需求添加：

- 技能命令：`AbilityCommandPlugin`，自动补充 `TargetingPlugin`；输入、AI 和 UI 写入游戏命令队列。
- 只需索敌：`TargetingPlugin`。移动系统放在 `GameplaySet::Movement`，需要同 tick 执行的命令在 GAS `RequestProducers` 等早于 Prepare 的阶段生产。
- 平滑表现：`PositionPresentationPlugin`，逻辑位置与显示 Transform 分离；相机与渲染实体由游戏创建。
- 空间随机：显式初始化 `SpatialRandom`，使用游戏自己的种子，保持 GAS 概率流独立。
- 自定义额外成本：在主程序的模拟装配处使用 `SimulationPlugin::new(settings).with_additional_costs::<GameCosts>()`，并在游戏插件初始化所需资源。

RTS 的选择与移动、MOBA 的阵营和技能输入、ACT 的蓄力和攻击命中可以各自建立游戏模块，不需要为了这些业务调整基础启动器。具体命令和生命周期边界见 [07 — 架构与插件组合](07-architecture-and-composition.md) 与 [08 — 游戏命令](08-game-commands.md)。

## 接入自己的配置

决定使用当前配置工具链时，编辑 Excel/Schema 并导表，再在游戏初始化中调用 `load_tables`、`compile_catalog`，成功后保存 Catalog、创建 Actor、初始化属性并授予技能。配置错误应传播给游戏启动流程或展示为明确的加载失败，避免继续运行部分场景。

`application::default_config_directory()` 只提供默认路径，不主动读文件；`GameResult` 是通用结果类型。自己的配置目录参数由游戏按需求设计，基础启动器不提供 `--config`。

保留配置演示时，可参考 `examples/configured_game/main.rs`、`demo.rs` 与 `demo/`。它读取示例表、创建 Laser 场景，并在无窗口模式执行固定 13 tick 验收：

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run --example configured_game
cargo run --example configured_game -- --headless --config "./assets/config"
cargo test --all-features --examples
cargo test --no-default-features --examples
```

这里的 `--headless` 是演示验收模式；基础程序的同名参数用于持续运行自己的游戏。演示的 `LaserReport`、`run_headless` 和 `run_windowed` 不属于模板库 API；迁移旧调用方时，将演示用途改为执行 example，自己的游戏运行器使用共享模拟装配。

配置流程与扩展规则见 [01 — 配置所有权](01-configuration.md) 和 [03 — Excel 配置规则](03-gas-configuration.md)。
