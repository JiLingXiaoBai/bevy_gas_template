# 07 — 架构与插件组合

## 模块所有权

两个包保持单向依赖：游戏模板依赖 GAS，GAS 不依赖模板。模板内部按职责组合：

```text
main                → application + game::GamePlugin
application         → gameplay::GameplaySchedulePlugin + bevy_gas
game                → 按游戏需要组合 config / gameplay / bevy_gas
examples/configured_game → application + config + gameplay
config              → gameplay::targeting 的纯定义 + bevy_gas 的定义与注册接口
gameplay            → bevy_gas 的公开契约和阶段
```

`application` 只提供共享模拟装配、默认配置路径和通用结果类型。它不加载配置或创建场景。
`src/game.rs`、`src/game/` 是游戏自己的业务入口，初始 GamePlugin 为空，由 main 注册。
`gameplay` 提供可复用能力，`config` 构造 GAS 定义和游戏索敌定义，两者不启动演示。

`application` 公开 `SimulationPlugin`、`SimulationSettings`、`SimulationSettingsError`、
`GameResult` 和 `default_config_directory`。LaserReport、run_headless、run_windowed 已从
application 和 gameplay 的库门面移除，演示实现与验收测试位于 `examples/configured_game/`。
旧调用方执行 `cargo run --example configured_game -- --headless` 进行演示验收；自己的游戏
通过共享模拟与 GamePlugin 开发。完整开始步骤见 [09 — 从模板开始新游戏](09-starting-a-game.md)。

基础启动器不依赖 examples；整个 examples 可以删除，tests 也可独立删除但建议保留基础回归。
不要通过 `#[path]` 等方式让 src 或基础测试反向引用演示。`src/application/` 与
`config/generated/` 仍属于基础代码，删除示例时保留。

## 独立的游戏调度

`GameplaySet` 只描述阶段名称；`GameplaySchedulePlugin` 注册阶段顺序，不创建索敌队列、索引、
渲染或 GAS 运行时资源。自行创建 App 且尚未安装 SimulationPlugin 时，需要移动但不需要索敌可显式组合：

```rust
use bevy::prelude::*;
use bevy_gas::GameplayAbilitySystemPlugin;
use bevy_gas_template::gameplay::{GameplaySchedulePlugin, GameplaySet};

fn configure_game(app: &mut App) {
    app.add_plugins((GameplayAbilitySystemPlugin, GameplaySchedulePlugin));
}
```

移动系统放在 `GameplaySet::Movement`，位于 GAS `EffectTicks` 之后、`AbilityTasks` 之前；
通过 Commands 提交的移动会在有序的后续阶段可见。集合内多个系统的业务顺序仍由游戏显式指定。

`TargetingPlugin` 自动检查并安装 `GameplaySchedulePlugin`，保持既有安装方式。
若同时显式安装，应先添加 Schedule 再添加 Targeting；不重复显式添加唯一插件。
`PositionPresentationPlugin` 可独立于 GAS 使用，因此不自动安装游戏调度或 GAS。
完整阶段图与索敌可见性见 [05 — 空间与索敌](05-spatial-targeting.md#fixedupdate-时序与边界)。

## 可独立选择的构建功能

| 构建方式 | 窗口与渲染 | 完整配置校验与 CLI |
| --- | --- | --- |
| 默认 / `--all-features` | 开启 | 开启 |
| `--no-default-features` | 关闭 | 关闭 |
| `--no-default-features --features config-validation` | 关闭 | 开启 |
| `--no-default-features --features windowed` | 开启 | 关闭 |

窗口代码按 feature 编译。关闭 `windowed` 后，基础运行时依赖不包含渲染、音频或窗口后端；
仅传入 `--headless` 不会改变编译依赖。配置读取与编译、逻辑位置、Transform 插值、索敌和
GAS 执行在基础构建中可用。导表脚本仅启用无窗口的配置校验构建。

基础 main 在启用 `windowed` 且没有 `--headless` 时安装 DefaultPlugins，显示无相机、无场景的
空窗口；关闭窗口退出。其他情况下使用 MinimalPlugins 与 ScheduleRunnerPlugin 持续运行。
`--ticks N` 只允许无窗口模式且 N 必须为正数；达到该 fixed tick 数后发出退出请求，供启动检查使用。
退出在帧末处理，同一帧可能仍执行追赶 tick，因此不是精确步进或回放接口。不传该参数时持续运行。
`--tick-hz HZ` 和 `--seed U64` 覆盖共享模拟的默认设置；基础 main 不接收 `--config`。

配置演示另有 `--headless` 和 `--config` 参数；关闭 windowed feature 时仍须显式指定演示的
`--headless`，否则报告窗口功能未启用。演示保持自己的固定验收时长，与基础 runner 分离。

新增业务放在 game 下所属领域，通过 Component/Resource/System/Plugin 组合；演示装配放在
examples。可复用能力不应为了获取演示实体或路径而反向依赖 game 或 examples。

## 共享模拟装配

`application::SimulationPlugin` 统一安装 GAS、`GameplaySchedulePlugin`、`Time<Fixed>` 与
GAS `Random`。默认 tick 为 50 ms（20 Hz），默认随机种子与 GAS 一致；窗口示例、无窗口验收
和新的重放测试使用同一入口。插件不创建场景、不加载配置、不安装索敌、命令或表现。

```rust
use bevy::prelude::*;
use bevy_gas_template::application::{SimulationPlugin, SimulationSettings};
use bevy_gas_template::gameplay::commands::AbilityCommandPlugin;

fn simulation_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::new(SimulationSettings::default().with_random_seed(42)),
        AbilityCommandPlugin,
    ));
    app
}
```

使用 `SimulationSettings::new(Duration)` 或 `try_from_hz(f64)` 修改 tick，非法值返回
`SimulationSettingsError`。配置在安装时生效；运行后修改该 Resource 不会自动重置时钟或 RNG。
空间随机仍由游戏单独初始化，避免改变 GAS 概率流。

安装顺序为时钟基础插件 → Simulation → 可选游戏插件。Simulation 已安装 GAS，不能再次
添加 GAS 插件组。需要背包等额外消耗时，使用
`SimulationPlugin::new(settings).with_additional_costs::<GameCosts>()` 替代默认插件，并显式
初始化游戏消耗资源。索敌和技能命令仍可以直接与原 GAS 插件组合，不要求使用应用层装配。

基础程序已经安装共享模拟和 GamePlugin；游戏插件按需追加配置、索敌、输入与场景。
`examples/configured_game/` 使用同一模拟装配，并在自己的 headless 实现中进行 Laser 的
13 tick 验收，检查命令终态和最终数值。它的停止条件、结果类型和演示场景都不进入库 API。

## 多类型游戏的接入验证

- `examples/multi_unit_commands.rs` 展示多单位命令：游戏 FIFO 接收移动与施法，
  移动目标由游戏系统执行，技能意图交给技能命令适配器，命中／效果仍由 GAS 结算。
- `examples/charged_ability.rs` 展示游戏侧蓄力：开始蓄力尚未激活 GAS，释放才提交正式技能；
  取消、超时、重复释放和实体销毁不会重复扣费。它不增加 GAS 原生等待外部信号任务，也不改变
  已激活技能的自动提交时机。示例使用明确的实体目标。

这两个例子不依赖窗口或导表产物，可分别运行和测试：

```bash
cargo run --no-default-features --example multi_unit_commands
cargo run --no-default-features --example charged_ability
cargo test --no-default-features --example multi_unit_commands
cargo test --no-default-features --example charged_ability
```

阵营、导航、攻击碰撞与游戏动作继续由具体游戏定义。配置中的固定时间线和现有编译扩展边界
见 [01 — 配置所有权](01-configuration.md)；有新的动作或生命周期用例时再扩展对应 Schema，
本轮不改变已有表格式。

## 验证

基础配置、输入、空间、命令与模拟测试保留在 tests，不依赖 examples 或导表生成的二进制资产。
调度回归验证不安装索敌时的移动与 Commands 可见性、Targeting 自动安装和显式前置安装。
配置演示测试验证入口错误和真实技能执行结果，留在 examples/configured_game；保留 examples
时，导表后显式执行 `cargo test --all-features --examples` 与 `cargo test --no-default-features --examples`。
删除示例后继续检查基础构建、测试与有限 tick 启动，跳过示例命令。

默认与最小构建均检查 Clippy 和测试；`cargo tree --no-default-features --edges normal`
可检查运行时依赖，测试专用的 keyboard feature 不改变发布库的渲染依赖边界。

`tests/gameplay_test/simulation_test.rs` 比较同种子、同命令下逐 tick 的逻辑位置、属性、
活跃效果／任务数和命令结果，同时验证更换命令或种子会改变结果。这是相同版本和受控实体
创建顺序下的重放检查，不承诺跨平台锁步或完整存档恢复。

多单位综合测量使用 128 / 1024 单位，组合索敌、持续效果、等待任务和批量技能命令：

```bash
cargo test --release --no-default-features --test gameplay_test measure_multi_unit_effect_and_task_workloads -- --ignored --nocapture
```

测试报告 fixed tick 耗时、效果／任务峰值与已有收敛计数，不设置依赖机器性能的耗时断言。
比较时固定构建模式、机器和场景，修改存储或执行策略前先保留基线。

2026-10-20 本机 Windows、Rust 1.98.1 的一次 release 测量如下。每组先预热 1 tick，再运行
24 tick，每 6 tick 批量施法一次；开启收敛诊断，计时只包含 fixed schedule，不含场景构建、
队列入队与 tick 后断言。以下均值包含空闲 tick，不代表批量施法 tick 的峰值或帧率承诺。

| 单位数 | 命令总数 | 效果峰值 | 任务峰值 | 总耗时 / ms | 平均 tick / ms | 收敛耗时 / ms |
| --- | --- | --- | --- | --- | --- | --- |
| 128 | 512 | 192 | 256 | 11.463 | 0.478 | 7.207 |
| 1024 | 4096 | 1536 | 2048 | 457.807 | 19.075 | 428.001 |

本次 1024 单位场景约 93.5% 的 tick 耗时落在效果需求收敛，说明后续大规模 RTS 优化应优先
分析该路径。当前结果用于保留可重复运行的起点；修改 GAS 收敛策略前仍需多次测量与专门
Profiling，不能仅凭总均值推断复杂度或跨机器表现。
