# 模板库使用指南与 GAS 工作原理

本文面向使用 `bevy_gas_template` 开发游戏的程序员，覆盖从启动模板、加载 Excel 配置、创建角色、提交技能命令，到理解 GAS 标签、属性、效果、技能、任务及结算顺序的完整流程。示例和接口以当前两个仓库的源码为准，技术基线为 Rust 1.97.1+、edition 2024、Bevy 0.20.0。

模板承担游戏侧的配置、输入适配、空间查询与运行装配；`bevy_gas` 承担通用技能和效果结算。开发新游戏时，先从模板的 `GamePlugin` 添加业务，再按需要使用底层 GAS API。

## 阅读导航

- [1 两个仓库和开发入口](#1-两个仓库和开发入口)
- [2 安装与运行](#2-安装与运行)
- [3 插件组合与功能开关](#3-插件组合与功能开关)
- [4 GAS 的数据模型](#4-gas-的数据模型)
- [5 一个 fixed tick 内发生什么](#5-一个-fixed-tick-内发生什么)
- [6 配置从 Excel 到运行时](#6-配置从-excel-到运行时)
- [7 配置技能的完整接入](#7-配置技能的完整接入)
- [8 不依赖配置的技能示例](#8-不依赖配置的技能示例)
- [9 Gameplay Tags](#9-gameplay-tags)
- [10 Attributes 与 Aggregator](#10-attributes-与-aggregator)
- [11 Modifiers 与数值捕获](#11-modifiers-与数值捕获)
- [12 Gameplay Effects](#12-gameplay-effects)
- [13 Gameplay Abilities 与 ASC](#13-gameplay-abilities-与-asc)
- [14 Ability Tasks 与技能链](#14-ability-tasks-与技能链)
- [15 统一执行队列与反馈](#15-统一执行队列与反馈)
- [16 目标结果与游戏空间索敌](#16-目标结果与游戏空间索敌)
- [17 输入命令与表现](#17-输入命令与表现)
- [18 随机数 名称池与确定性](#18-随机数-名称池与确定性)
- [19 常见玩法的实现方式](#19-常见玩法的实现方式)
- [20 调试 验证与扩展边界](#20-调试-验证与扩展边界)

初次接入可先读 1—8，再运行示例；排查数值和时序问题可直接查 9—18。最后两节提供玩法实现建议、检查命令及源码索引。

## 1 两个仓库和开发入口

### 目录布局

默认 Cargo 依赖为：

```toml
bevy_gas = { path = "../bevy_gas" }
```

因此目录应类似：

```text
workspace/
├── bevy_gas/                  # GAS library
└── my_game/                   # Game created from bevy_gas_template
    ├── src/main.rs
    ├── src/game.rs
    ├── src/game/
    ├── src/application/
    ├── src/gameplay/
    ├── src/config/
    ├── config/
    └── examples/
```

游戏文件夹可以任意命名。Cargo 不会下载 path 依赖，必须另行准备兼容的同级 GAS 源码，或修改实际路径。修改本地 GAS 后，游戏下一次构建就会使用修改，无需提交、推送或重新发布 crate。path 依赖的源码内容不由 Cargo.lock 固定，团队和发布环境仍需记录所使用的两个仓库版本。

从 GitHub 模板创建的仓库独立维护，后续模板改动不会自动合并，Rust 包名也不会自动替换。重命名时同步 `Cargo.toml` 的 package/default-run、源码和测试中的 crate 导入、保留的示例及文档。

### 各目录应该放什么

| 位置 | 职责 | 典型修改 |
| --- | --- | --- |
| `src/main.rs` | 选择窗口或无窗口运行环境，解析基础启动参数 | 自定义启动参数、装配外部成本 Provider |
| `src/application/` | `SimulationPlugin`、模拟设置、通用结果与配置路径 | 统一窗口、测试、服务端的模拟设置 |
| `src/game.rs` 和 `src/game/` | 当前游戏自己的业务 | 单位、阵营、战斗规则、AI、相机、UI、关卡 |
| `src/gameplay/` | 可复用的命令、输入绑定、位置、索敌、空间随机 | 新命令适配、目标规则、游戏侧通用能力 |
| `src/config/` | 解码、校验、编译、Catalog 与技能授予映射 | 新表字段到运行时定义的映射 |
| `config/tables/` | Excel 数据及字段声明 | 数值、技能时间线、效果、索敌规则 |
| `config/defines/` | Luban 表登记、枚举、Bean | 新表、枚举、结构 |
| `config/generated/` | 自动生成 Rust 数据类型 | 通过导表生成，保留目录 |
| `assets/config/` | `.bytes` 和 manifest 配置包 | 通过导表生成并部署 |
| `examples/` | 可删除的演示 | 学习和验证，不作为游戏运行依赖 |
| `../bevy_gas/src/gas/` | GAS 通用机制 | 新原生任务、聚合或生命周期机制 |

依赖方向是游戏到 GAS。GAS 不认识 Excel、键盘、阵营、相机或空间树。模板的 `gameplay::spatial` 是内部实现，游戏通过公开的 `targeting` 使用空间查询。

新增业务模块遵循“门面文件 + 同名目录”，在门面显式重导出公开项。代码注释使用英文；知识库使用中文。运行时代码通过 `Result`、`?`、`match`、`let ... else` 处理失败，不使用 `unwrap()`、`expect()`、`panic!()`。

## 2 安装与运行

### 先运行基础程序

在模板或游戏仓库根目录执行：

```powershell
cargo fetch --locked
cargo run
```

默认打开空窗口。主程序已经安装 `SimulationPlugin` 和空的 `GamePlugin`，但没有加载 Excel、创建角色或相机。空窗口是预期行为，基础启动也不要求安装 Luban。

```powershell
cargo run -- --headless --ticks 3
cargo run --no-default-features -- --ticks 20
cargo run -- --headless --ticks 120 --tick-hz 60 --seed 42
```

| 参数 | 行为 |
| --- | --- |
| `--headless` | 无窗口持续运行 |
| `--ticks N` | 无窗口模式达到正数 N 个 fixed tick 后请求退出 |
| `--tick-hz HZ` | 设置固定模拟频率，默认 20 Hz；要求有限正数且能形成非零时间步 |
| `--seed U64` | 设置 GAS 随机种子；省略时使用固定默认种子 |

未启用 `windowed` 时自动无窗口运行。无窗口且不传 `--ticks` 时程序持续执行，可用 Ctrl+C 停止。退出请求在帧末处理，同帧可能继续追赶 tick，因此 `--ticks` 适合启动检查，不是精确离线步进接口。

`--headless` 只改变运行模式；要减少编译的窗口和渲染依赖，使用 `--no-default-features`。基础主程序没有 `--config` 参数，自己的加载参数应由游戏实现。

### 运行配置技能演示

当前 Luban 安装与导表脚本支持 Windows x64，需要 Git、PowerShell 7.2+、系统 .NET Runtime 8+ 和 PATH 中的 7-Zip。

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run --example configured_game
```

窗口演示按 Space 释放 Laser。无窗口验收：

```powershell
cargo run --example configured_game -- --headless
cargo run --no-default-features --features config-validation --example configured_game -- --headless
cargo run --example configured_game -- --headless --config "./assets/config"
```

该演示显式执行 13 次 `FixedUpdate`，检查目标 Health 从 500 变为 320、施法者 Mana 从 100 变为 80、活跃技能数归零。失败返回非零退出码。演示默认配置路径基于编译时工程根目录；部署时通过演示自己的 `--config` 指定位置。

基础程序和配置演示的 `--headless` 含义不同：基础程序持续模拟，演示执行有限次数并验收。

### 其他独立示例

以下命令在模板仓库执行，不需要导表：

```powershell
cargo run --no-default-features --example logical_position_targeting
cargo run --no-default-features --example ability_input_bindings
cargo run --no-default-features --example multi_unit_commands
cargo run --no-default-features --example charged_ability
```

在 GAS 仓库还可运行：

```powershell
cargo run --example ability_effect_flow
cargo run --example tag_registration
cargo run --example inventory_bomb
```

分别学习纯 GAS 技能流程、标签注册和游戏背包成本协议。可删除模板整个 `examples/`，保留 `src/application/` 与 `config/generated/`；基础库不反向依赖示例。

## 3 插件组合与功能开关

### 推荐的装配路径

模板主程序已经安装共享模拟。自己的 `GamePlugin` 按需加入命令、表现和业务系统即可：

```rust
use bevy::prelude::*;
use bevy_gas_template::gameplay::commands::AbilityCommandPlugin;
use bevy_gas_template::gameplay::position::PositionPresentationPlugin;

fn configure_game(app: &mut App) {
    app.add_plugins((AbilityCommandPlugin, PositionPresentationPlugin));
}
```

这段是装配函数示意，不是完整程序。在本仓库 `src/game/` 内，模板自身的导入应改用 `crate::gameplay::...`。

| 插件 | 自动提供 | 不会替游戏完成 |
| --- | --- | --- |
| `SimulationPlugin` | GAS、`GameplaySchedulePlugin`、固定时钟和 GAS 随机种子 | 配置加载、角色、索敌、输入、相机 |
| `GameplaySchedulePlugin` | 游戏阶段相对 GAS 的执行顺序 | 具体系统、队列、空间索引 |
| `AbilityCommandPlugin` | 技能命令消费与反馈，自动补充 `TargetingPlugin` | 设备输入、技能栏、选择单位 |
| `TargetingPlugin` | 索引同步、索敌请求和结果，缺少时补充调度插件 | GAS 本身、命中伤害、移动和阵营规则 |
| `PositionPresentationPlugin` | 逻辑位置到 Transform 的表现同步和插值 | 相机、Mesh、游戏移动、GAS |
| `GameplayAbilitySystemPlugin` | GAS 基础设施与运行时的插件组 | Bevy runner、游戏场景、配置和索敌 |

不要在已经使用 `SimulationPlugin` 的 `GamePlugin` 内再次安装 GAS。独立创建 App 时先添加 Bevy 的运行环境，再安装 `SimulationPlugin`；若自行组合底层插件，应明确保留所需的名称、标签、随机和属性注册资源。

`GameplayAbilitySystemRuntimePlugin` 是底层运行时插件，不等同于完整插件组。通常只在需要自定义装配时直接使用。显式安装调度、索敌及上层命令插件时，先装依赖插件，避免上层已自动安装后再次重复添加。

### 功能开关

| Cargo 选项 | 窗口依赖 | 完整配置校验和 gas-config |
| --- | --- | --- |
| 默认或 `--all-features` | 开启 | 开启 |
| `--no-default-features` | 关闭 | 关闭 |
| `--no-default-features --features config-validation` | 关闭 | 开启 |
| `--no-default-features --features windowed` | 开启 | 关闭 |

关闭 `config-validation` 后，生成类型、二进制解码、Catalog 编译和技能授予依然可用；只是没有 manifest/摘要/schema 检查及完整制作规则校验。必要的解码边界、引用和运行时合法性检查仍执行。

## 4 GAS 的数据模型

### 定义 规格和运行状态

GAS 将可共享的内容与每次运行的状态分离：

| 层次 | 典型类型 | 所有权和用途 |
| --- | --- | --- |
| 共享定义 | `Arc<GameplayAbility>`、`Arc<GameplayEffect>` | 技能规则、任务列表、修改器公式；多个角色共享 |
| 已授予规格 | `GameplayAbilitySpec` | 某角色获得的技能、等级、Handle 和活跃实例计数 |
| 已捕获效果规格 | `GameplayEffectSpec`、`ModifierSpec` | 某次施加时计算好的幅度、持续 tick、周期 tick |
| 活跃技能 | `ActiveGameplayAbility` | 一次激活实体上的 Component，持有上下文和状态；任务另存实体 |
| 活跃效果 | `ActiveGameplayEffect` | 存放于目标 `ActiveGameplayEffects` 中，持有层数、计时和抑制状态 |
| 等待任务 | `AbilityTask` | 独立 ECS 任务实体，关联父技能 |

同一技能定义被授予两个角色，会得到两个角色各自的规格。多次激活得到不同活跃实例。持续效果按定义的 `Arc` 身份匹配叠层，两个内容相同但独立构造的 `Arc<GameplayEffect>` 不属于同一个定义。

### 角色应该挂哪些组件

完整 GAS 角色使用 `GameplayAbilitySystemBundle`：

| Bundle 字段 | Component | 作用 |
| --- | --- | --- |
| `ability_system` | `AbilitySystemComponent` | 已授予技能和活跃实例索引 |
| `tags` | `GameplayTagContainer` | 当前角色状态标签及引用计数 |
| `attributes` | `AttributeSet` | Health、Mana 等属性与聚合状态 |
| `active_effects` | `ActiveGameplayEffects` | 该角色上的持续效果 |

创建 Bundle 不会自动初始化 Health/Mana，也不会授予技能。游戏仍需登记属性、初始化值、授予技能，并按需要添加 `LogicalPosition`、`Targetable`、阵营等组件。

只做分类的实体可以单独挂标签，只存数值的实体可以单独挂属性；这些组件不会反向自动补齐活跃效果容器。需要持续效果时应提供完整的目标组件组合。

### 不要混用不同 ID

| ID | 范围 | 用法 |
| --- | --- | --- |
| `AbilityId(i32)`、`EffectId(i32)` | 游戏配置 | 存档、策划引用和跨运行识别 |
| `AbilitySpecHandle` | 单个角色 ASC 内 | 激活该角色已经获得的技能 |
| `ActiveAbilityHandle` | 当前 World 的 Entity | 结束或取消某次技能实例 |
| `ActiveEffectHandle` | 所属 World 的目标容器、槽位与世代 | 移除或追踪某次持续效果 |
| `GameplayTag`、`AttributeId` | 当前注册环境 | 高效运行时查询 |
| 命令、索敌、执行 Request ID | 各自队列 | 关联本次请求与反馈 |

配置 ID 1001 不能直接当 `AbilitySpecHandle`。运行时内部索引也不适合作为长期存档 ID；加载存档时按配置 ID/稳定名字重新注册、授予和绑定。

### 公共参数的边界

`EffectSystemParams` 用于效果准备、施加和移除，包含效果所需的查询和资源。`AbilitySystemParams<P>` 在它之上加入 ASC、活跃技能、Commands 和额外成本 Provider。纯效果系统优先声明前者，避免让简单伤害逻辑依赖完整技能编排。

`prelude` 只导出常用项。错误类型、完整任务定义和诊断 API 可从 `bevy_gas::gas::<领域>` 或 crate 根显式导入；不要假定 `prelude::*` 包含全部 API。

## 5 一个 fixed tick 内发生什么

### 完整调度顺序

启用模板调度和技能命令插件后，核心顺序如下。方括号中的阶段属于可选的游戏插件：

```text
FixedUpdate
  EffectTicks
    duration -> tag convergence -> period -> tag convergence
  Movement
  AbilityTasks
  RequestProducers
  [AbilityCommandSet::Prepare]
  [SpatialSync]
  [Targeting]
  [AbilityCommandSet::Submit]
  PreGameplayConvergence
  GameplayResolve
  UpdateEffectTagRequirements
  Cleanup
  RecalculateAttributes
  [AbilityCommandSet::Results]
```

- `EffectTicks` 先推进已存在效果的时长，再执行周期；到期并被移除的效果不会在同 tick 再 pulse。
- `Movement` 更新权威逻辑位置，先于任务、请求和索敌。
- `AbilityTasks` 推进等待任务；到期动作可以生成本 tick 要处理的效果或技能请求。
- `RequestProducers` 是游戏输入、AI、命中等请求生产系统的公共接入阶段。
- 命令 Prepare 捕获待执行命令，SpatialSync 更新索引，Targeting 取得目标，Submit 按命令顺序提交 GAS。
- `PreGameplayConvergence` 在 resolver 前处理外部标签变化引起的效果抑制或移除。
- `GameplayResolve` 完整消费统一 FIFO；技能激活与效果应用共享顺序。
- 最后收敛效果条件、清理结束技能、重算脏属性，并发布命令反馈。

集合内多个游戏系统如果有业务先后关系，仍要显式 `.chain()`、`.before()` 或 `.after()`。仅放进同一个 Set 并不规定彼此顺序。

### 同 tick 与下一 tick 的边界

在 resolver 前进入 GAS 队列的请求，本 tick 处理。resolver 之后产生的请求，等下一 tick。通过默认 `GameplayResolve` 队列执行的新激活发生在 `AbilityTasks` 之后，因此激活时创建的 `WaitTicks(1)` 在下一次任务阶段完成。

启动阶段的 `Instant` 动作是特殊的同步执行路径：立即效果和子技能启动在当前激活内按定义顺序完成，不需要再等公共 FIFO。通过 Commands 触发的 Observer 仍受 deferred 应用时机影响，不能当成同步返回值。

游戏需要读取本 tick 稳定的最终属性时，将读取系统放在 `RecalculateAttributes` 之后。表现系统通常在 Update 读取固定模拟的结果，不应反过来决定战斗的逻辑位置。

### tick 不是秒

持续、周期、冷却和任务等待都以 fixed tick 为单位。默认 20 Hz 时，1 tick 是 0.05 秒，20 tick 是 1 秒，180 tick 是 9 秒。改成 60 Hz 后，同一个 180 tick 变为 3 秒，表格不会自动换算。

确定性测试可以像现有示例一样调用 `world.run_schedule(FixedUpdate)` 明确推进 N 次。这与基础启动器的 `--ticks` 退出检查不同。显示剩余秒数时由 UI 使用当前 fixed timestep 转换，玩法内部继续使用整数 tick。

## 6 配置从 Excel 到运行时

### 七张表各自负责什么

| 文件 | 主键 | 内容 |
| --- | --- | --- |
| `gas.tag.xlsx` | name | 标签名称与说明 |
| `gas.attribute.xlsx` | name | 属性名、Hot/Cold 区域 |
| `gas.modifier.xlsx` | id | 属性、操作、Flat/LinearLevel、base、per_level |
| `gas.effect.xlsx` | id | 时长、周期、概率、标签和 `modifier_ids` |
| `gas.ability.xlsx` | id | 等级、标签、成本、冷却、索敌 ID、`task_ids`、额外成本 |
| `gas.ability_task.xlsx` | id | `at_tick`、动作类型、目标范围和效果 ID |
| `gas.targeting.xlsx` | id | 选择形状、标签/距离过滤、排序、数量限制 |

属性表声明属性身份，不声明角色初始生命。角色创建时仍要调用 `initialize_attribute`。

Excel 的 `##var`、`##type`、`##group`、`##` 分别用于字段名、类型、分组、说明；数据行首列留空。可空值留空，不填写字符串 `null`。字段类型和列表分隔规则以当前表头、Schema 为准。

`task_ids`、`modifier_ids` 是分号分隔的有序引用列表。同一父列表不允许重复 ID，不同父定义可以共享子行。调整 Excel 物理行顺序不会替代这些列表中的顺序。

### 额外成本是内联列表

当前没有第八张额外成本表。`ability.additional_costs` 是 `AbilityAdditionalCost` Bean 列表：

```text
5001,1;5002,3
```

表示资源 5001 消耗 1 个、资源 5002 消耗 3 个。空单元格表示没有额外成本。资源 ID 使用完整范围的 `i32`，正负和零是否合法由游戏决定；数量必须在 `1..=u32::MAX`。

重复项保留，例如 `5001,1;5001,1` 会原样交给 Provider，整批需要两个 5001。编译器检查同技能同资源合计是否溢出，但不合并或重排输出。它也不会创建背包或判断 5001 是什么物品。

### 导表和加载的阶段

```text
Excel + XML + Rust generation templates
  -> Luban export into staging directory
  -> generated Rust + seven binary tables
  -> build candidate gas-config
  -> manifest + package validation + GAS compilation
  -> publish config/generated and assets/config
  -> load_tables
  -> compile_catalog
  -> GameplayCatalog
  -> grant_ability on each actor
```

统一发布入口是 `config/export.ps1`。它在暂存工程中编译候选生成代码，使用实际配置编译器读取、校验新二进制，全部通过后才发布。生成 Rust、七个 `.bytes` 和 manifest 必须配套；不要手工修改生成代码或只重写 manifest 来绕过失败。

导表期间不要并发启动读取配置：两个输出目录的替换不是对所有读者同时可见的瞬时切换。基础启动程序不读表，不受这一运行时配置读取限制影响。

### 加载和编译 API

| API | 输入与结果 |
| --- | --- |
| `load_tables(directory)` | 读取配置目录，返回生成的 `Tables` 或 `ConfigError` |
| `compile_catalog(&tables, &mut world)` | 注册和构造共享定义，返回 `GameplayCatalog` |
| `catalog.ability(AbilityId)` | 返回技能定义、最大等级、独立索敌定义 |
| `catalog.effect(EffectId)` | 返回该 ID 唯一的共享效果 Arc |
| `catalog.attribute(name)` / `tag(name)` | 返回当前注册环境中的属性/标签 |
| `grant_ability(asc, bindings, catalog, id, level)` | 授予配置技能，返回当前角色 Handle |
| `revoke_ability(asc, bindings, id)` | 撤销配置技能并清理映射，活动实例存在时拒绝 |

World 必须先安装 GAS。`compile_catalog` 先准备引用、公式和时间线，再克隆名称池、标签表、属性表，在私有快照内完成注册与全部定义构造；成功后统一提交注册表并返回 Catalog。发生 `Err` 时，World 的原注册表和已有 Catalog 保持不变。调用方只在成功后插入 Catalog。

同名属性的 Hot/Cold 区域必须一致，同名标签的完整继承关系必须一致。编译器按稳定顺序登记，而不是依赖 HashMap 或 Excel 行顺序。每个 EffectId 只创建一个 Arc，成本、冷却、任务引用复用它；不同 EffectId 不按内容去重。

当前适合启动时加载，未提供战斗中 Catalog 热替换。给现有战斗直接换 Arc 可能改变叠层身份、标签和 Handle 映射，不能把重新编译当作现成的热更新协议。

### 校验边界

启用 `config-validation` 时，加载器验证 manifest、模板与结构版本、预期表集合、文件大小、BLAKE3 摘要和整包摘要，再解码同一次读取的数据。编译器额外执行完整制作规则，包括等级、时间线、成本和冷却等约束。

最小构建不读取 manifest，但保留二进制边界、引用、实际使用参数和运行时构建检查。两种模式都限制单表 64 MiB、表数据合计 256 MiB；启用校验时清单上限为 1 MiB。摘要用于一致性检查，不是发布者签名。

`ConfigError::kind()` 可匹配错误类别，`location()` 定位文件、表行、字段或操作，`message()`/Display 显示原因。恢复逻辑应匹配结构化类型，不解析错误字符串。

### 配置支持的功能小于 GAS 底层能力

当前配置公式有：

```text
Flat        = base
LinearLevel = base + per_level * (level - 1)
```

时间线按 `(at_tick, task_ids 中的位置)` 排序；相同 tick 的动作合并为一个 Batch。tick 0 使用 Instant，正 tick 使用 WaitTicks，各组从激活开始并行等待。完整制作校验要求恰好一个 EndAbility，且在最后执行。

配置动作目前只包含 ApplyEffect 和 EndAbility。ApplyEffect 的目标范围是 Primary 或 AllCaptured；EndAbility 的 target_scope 应为 None，effect_id 留空。完整制作校验要求属性成本为 Instant、非周期、概率 1、每属性唯一的 Add 且各允许等级均为负；冷却为正持续时间、非周期、概率 1、无 Modifier 且具有非空 granted tags。配置效果统一使用 `StackingPolicy::non_stacking()`，没有叠层策略字段。自定义公式、EmitEvent、技能链、复杂免疫/ongoing 规则等可以用 Rust API 表达；想由策划配置时还需扩展 Schema 和编译映射。

## 7 配置技能的完整接入

下面三个函数片段展示接入位置。代码位于游戏的二进制或 example 中时可使用 `bevy_gas_template::...`；放进模板库内部模块时按项目规范使用 `crate::...` 导入。

### 创建运行环境并加载 Catalog

```rust
use bevy::prelude::*;
use bevy_gas_template::application::{GameResult, SimulationPlugin, default_config_directory};
use bevy_gas_template::config::{compile_catalog, load_tables};
use bevy_gas_template::gameplay::commands::AbilityCommandPlugin;

fn create_configured_app() -> GameResult<App> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SimulationPlugin::default(),
        AbilityCommandPlugin,
    ));
    let tables = load_tables(default_config_directory())?;
    let catalog = compile_catalog(&tables, app.world_mut())?;
    app.insert_resource(catalog);
    Ok(app)
}
```

该函数只返回已装配的 App；调用者继续创建场景、注册业务系统并运行。默认配置目录来自编译时路径，正式打包时应传入自己的部署目录。

### 初始化角色并授予配置技能

```rust
use bevy::prelude::*;
use bevy_gas::{AttributeIdManager, GameplayAbilitySystemBundle};
use bevy_gas_template::application::GameResult;
use bevy_gas_template::config::{AbilityId, ConfiguredAbilities, GameplayCatalog, grant_ability};
use bevy_gas_template::gameplay::position::LogicalPosition;

fn spawn_configured_caster(world: &mut World) -> GameResult<Entity> {
    let catalog = world
        .get_resource::<GameplayCatalog>()
        .ok_or("missing catalog")?;
    let manager = world
        .get_resource::<AttributeIdManager>()
        .ok_or("missing attribute registry")?;
    let mana = catalog.attribute("Mana").ok_or("missing Mana")?;
    let mut actor = GameplayAbilitySystemBundle::default();
    actor
        .attributes
        .initialize_attribute(manager, mana, 100.0, None)?;
    let mut bindings = ConfiguredAbilities::default();
    grant_ability(
        &mut actor.ability_system,
        &mut bindings,
        catalog,
        AbilityId(1001),
        5,
    )?;
    Ok(world
        .spawn((actor, bindings, LogicalPosition(Vec3::ZERO)))
        .id())
}
```

技能 1001 消耗 Mana，所以这里只初始化 Mana。可受伤的角色还应初始化 Health 等业务属性。目标实体要有 `LogicalPosition`、`Targetable` 和所需属性；创建完整目标 Bundle 最容易保证持续效果需要的组件齐全。

`ConfiguredAbilities` 只保存当前角色配置 ID 到 Handle 的映射，必须和传入的 ASC 同属一个实体。它不会给角色添加输入绑定，也不会安装索敌或 UI。

### 提交符合配置规则的命令

```rust
use bevy::prelude::*;
use bevy_gas_template::application::GameResult;
use bevy_gas_template::config::{AbilityId, ConfiguredAbilities, GameplayCatalog};
use bevy_gas_template::gameplay::commands::{AbilityCommand, GameCommandId, GameCommandQueue};
use bevy_gas_template::gameplay::position::LogicalPosition;
use bevy_gas_template::gameplay::targeting::TargetingInput;
use std::sync::Arc;

fn queue_configured_laser(
    source: Entity,
    target: Entity,
    position: &LogicalPosition,
    bindings: &ConfiguredAbilities,
    catalog: &GameplayCatalog,
    queue: &mut GameCommandQueue<AbilityCommand>,
) -> GameResult<GameCommandId> {
    let id = AbilityId(1001);
    let handle = bindings.handle(id).ok_or("Laser is not granted")?;
    let ability = catalog.ability(id).ok_or("Laser is not configured")?;
    let input = TargetingInput::new(position.0, Vec3::X).with_explicit_target(target);
    Ok(queue.push(AbilityCommand::targeted(
        source,
        handle,
        input,
        Arc::clone(ability.targeting()),
    ))?)
}
```

系统中通过 `ResMut<GameCommandQueue<AbilityCommand>>` 取得队列，将生产系统放在 `RequestProducers` 或其他明确早于命令 Prepare 的阶段。`push` 成功返回的是命令 ID；最终是否激活成功要读 `AbilityCommandResult`。

若使用 `AbilityCommand::direct(source, handle, target)`，表示目标已经确定，不再执行配置中的距离和标签索敌过滤。需要沿用配置规则时使用上面的 targeted 路径。

### Laser 的实际数据和时间线

当前初始配置为：

| 项目 | 数据 |
| --- | --- |
| 技能 | 1001 Laser，最大 5 级，禁止并发实例，State.Stunned 阻止激活 |
| 消耗 | Effect 2001：Instant，Mana Add -20 |
| 冷却 | Effect 2002：180 tick，授予 Cooldown.Laser |
| 伤害 | Effect 2003：Health Add，`-100 + (-20) × (level - 1)` |
| 索敌 | 3001：显式目标，排除自己，要求 AttributeSet，最大距离 20，最多 1 个 |
| 动作 | tick 12 先应用 Primary 伤害，再 EndAbility |

演示双方初始 Health 500、Mana 100；施法者在原点，目标距离为 5，技能为 5 级。

| 相对 tick | 过程 | 可观察结果 |
| --- | --- | --- |
| 0 | 消费命令、索敌、GAS 激活、自动 Commit，建立等待任务 | Mana 80；冷却开始；目标 Health 500 |
| 1—11 | 推进任务及冷却 | 尚未执行 Laser 伤害 |
| 12 的任务阶段 | Batch 先将伤害入队，再标记 EndAbility | 已入队请求保留 |
| 12 的 resolver 与清理阶段 | 结算 -180 伤害，移除结束实例和任务 | Health 320、活跃技能数 0 |
| 后续 tick | 冷却继续自己的寿命 | 技能结束不会删除冷却 |

无窗口演示执行 tick 0 到 12，共 13 次 schedule。默认 20 Hz 时伤害延迟为 0.6 秒，冷却为 9 秒。捕获目标后，tick 12 不会自动重查距离；需要“命中时仍在范围内”的设计，应在游戏命中系统里另行校验。

配置技能 1002 InventoryBomb 使用 Mana 成本和内联资源 `5001,1`，但配置演示没有安装背包 Provider。它用于验证成本接入，不会自动生成炸弹实体。参考 [额外成本测试](../tests/config_test/additional_cost_test.rs) 和 [GAS 背包示例](../../bevy_gas/examples/inventory_bomb.rs)。

## 8 不依赖配置的技能示例

下面是完整的无窗口程序，可保存为模板 `examples/guide_quickstart.rs`，执行 `cargo run --no-default-features --example guide_quickstart`。它只使用 Rust 构造定义，运行 tick 0—3，检查消耗、延迟伤害和显式结束。该文件名是复制示例时使用的名字，仓库内现成示例见第二节。

```rust
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationContext, AbilitySystemComponent, AbilityTaskDef, AbilityTaskOnFinishedDef,
    AttributeId, AttributeIdManager, AttributeIdRegister, AttributeRegion, AttributeSet,
    EffectDurationTicks, EffectTags, GameplayAbility, GameplayAbilitySystemBundle, GameplayEffect,
    GameplayExecutionQueue, Modifier, ModifierMagnitude, ModifierOperation, StackingPolicy,
};
use bevy_gas_template::application::{GameResult, SimulationPlugin};
use std::sync::Arc;

fn register_attributes(mut ids: AttributeIdRegister) -> GameResult<(AttributeId, AttributeId)> {
    let health = ids.request_or_register_attribute_id("Health", AttributeRegion::Hot)?;
    let mana = ids.request_or_register_attribute_id("Mana", AttributeRegion::Hot)?;
    Ok((health, mana))
}

fn instant_delta(id: AttributeId, value: f32) -> Arc<GameplayEffect> {
    Arc::new(GameplayEffect::new(
        vec![Modifier::new(
            id,
            ModifierOperation::Add,
            ModifierMagnitude::Flat(value),
        )],
        EffectDurationTicks::Instant,
        None,
        1.0,
        StackingPolicy::non_stacking(),
        EffectTags::new(Vec::new(), Vec::new()),
    ))
}

fn main() -> GameResult<()> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimulationPlugin::default()));
    app.finish();
    app.cleanup();
    let (health, mana) = app
        .world_mut()
        .run_system_once(register_attributes)
        .map_err(|error| format!("attribute setup failed: {error:?}"))??;

    let ability = Arc::new(
        GameplayAbility::default()
            .with_cost(instant_delta(mana, -20.0))
            .with_startup_tasks(vec![
                AbilityTaskDef::wait_ticks(
                    2,
                    AbilityTaskOnFinishedDef::ApplyGameplayEffectToTarget {
                        effect: instant_delta(health, -30.0),
                    },
                ),
                AbilityTaskDef::wait_ticks(3, AbilityTaskOnFinishedDef::EndAbility),
            ]),
    );

    let manager = app
        .world()
        .get_resource::<AttributeIdManager>()
        .ok_or("missing attribute manager")?;
    let mut caster = GameplayAbilitySystemBundle::default();
    caster
        .attributes
        .initialize_attribute(manager, mana, 50.0, None)?;
    let handle = caster.ability_system.give_ability(ability, 1)?;
    let mut target = GameplayAbilitySystemBundle::default();
    target
        .attributes
        .initialize_attribute(manager, health, 100.0, None)?;
    let source = app.world_mut().spawn(caster).id();
    let target = app.world_mut().spawn(target).id();

    {
        let mut queue = app
            .world_mut()
            .get_resource_mut::<GameplayExecutionQueue>()
            .ok_or("missing gameplay queue")?;
        let chain = queue.new_root_chain(handle);
        let context = AbilityActivationContext::direct(source, chain);
        queue.push_activation(source, target, handle, context)?;
    }
    for _ in 0..=3 {
        app.world_mut().run_schedule(FixedUpdate);
    }

    let world = app.world();
    let manager = world
        .get_resource::<AttributeIdManager>()
        .ok_or("missing attribute manager")?;
    let remaining_mana = world
        .get::<AttributeSet>(source)
        .ok_or("missing caster attributes")?
        .evaluate_attribute(manager, mana)?
        .ok_or("missing Mana")?
        .current();
    let remaining_health = world
        .get::<AttributeSet>(target)
        .ok_or("missing target attributes")?
        .evaluate_attribute(manager, health)?
        .ok_or("missing Health")?
        .current();
    let asc = world
        .get::<AbilitySystemComponent>(source)
        .ok_or("missing ASC")?;
    let active_count = asc
        .find_ability_spec(handle)
        .ok_or("missing granted ability")?
        .get_active_count();
    if remaining_mana != 30.0 || remaining_health != 70.0 || active_count != 0 {
        return Err("unexpected ability result".into());
    }
    Ok(())
}
```

这里直接运行真实 FixedUpdate schedule，没有等待墙上时钟。tick 0 激活并支付；tick 2 伤害；tick 3 结束。等待 2 和等待 3 是从激活开始计时，不是等待 2 后再等待 3。

更完整的标签、冷却和日志演示见 [ability_effect_flow.rs](../../bevy_gas/examples/ability_effect_flow.rs)。注册标签和属性通常拆为有序的初始化系统，因为两个 Register 参数都会可变借用名称池，不应在同一个 System 中直接声明相互冲突的注册参数。

## 9 Gameplay Tags

### 使用方式

标签用来表达状态和规则，例如 `State.Control.Stunned`、`Ability.Laser`、`Cooldown.Laser`。它适合高频布尔条件查询，不承担数值属性或技能定义 ID 的职责。

```rust
use bevy_gas::{
    GameplayTag, GameplayTagContainer, GameplayTagError, GameplayTagManager, GameplayTagRegister,
};

fn register_stun(mut tags: GameplayTagRegister) -> Result<GameplayTag, GameplayTagError> {
    tags.request_or_register_tag("State.Control.Stunned")
}

fn apply_stun_tag(
    owned: &mut GameplayTagContainer,
    manager: &GameplayTagManager,
    stun: GameplayTag,
) -> Result<(), GameplayTagError> {
    owned.add_tag(&stun, manager)
}
```

`request_or_register_tag` 自动登记祖先。同一 World 中先统一注册，再在业务中保存返回的 `GameplayTag`，不必每次判断都重新解析字符串。

容器常用接口为 `add_tag`、`add_tags`、`remove_tag`、`remove_tags`、`has_tag`、`has_all`、`has_any`。`TagRequirements::new(require_all, ignore_any)` 表达“全部必需标签存在，并且任意禁止标签都不存在”。

### 层级和引用计数原理

全局每个标签分配一个位。当前上限 512，包括自动建立的父标签，位图为 8 个 `u64`。登记 `State.Control.Stunned` 会占用 State、State.Control 和完整名称的位。

每个实体对标签保存直接引用数 explicit 和总引用数 total。添加子标签同时增加它及祖先的 total，因此 `has_tag(State)` 也能识别眩晕状态。

如果两个效果都授予 Stunned，引用为 2；一个效果结束只移除自己的一份，另一个仍然维持眩晕。只有子标签引用时，直接移除父标签不会取消子标签贡献，因为父标签没有对应的 explicit 引用。

每个位的总引用上限为 `u16::MAX`，新增引用会超过上限时返回错误。批量添加先验证整批，重复项和共享祖先也计数，溢出不会只添加一部分。不要在效果失效时使用“清空全部状态”来代替成对移除引用。

`tag_bits_from_tags` 只收集输入标签的位；`tag_bits_from_tags_with_manager` 会扩展祖先。构造自定义匹配逻辑时要明确使用哪一种，避免把“具有共同父类”误当成“同一个具体状态”。

底层注册器负责身份和容量，游戏命名规范由配置层维护。标签 ID 是注册环境内的索引，不适合跨 World 或跨配置版本持久化。

详见 [标签模块](../../bevy_gas/.docs/03-gameplay-tags.md) 与 [标签容器实现](../../bevy_gas/src/gas/gameplay_tags/container.rs)。

## 10 Attributes 与 Aggregator

### 注册和初始化分两步

先通过 `AttributeIdRegister::request_or_register_attribute_id(name, region)` 声明全局属性身份，再对每个实体的 `AttributeSet` 初始化具体值。全局有 Health 不代表每个实体都已初始化 Health。

当前布局：

| 区域 | 容量 | 用途 |
| --- | --- | --- |
| Hot | 32 | AttributeSet 内联数组，适合频繁读写的属性 |
| Cold | 224 | Box 内的固定数组，承载低频属性 |
| 合计 | 256 | 全局注册容量 |

Hot/Cold 是显式区域，不是“前 32 个 ID 自动 Hot”。区域容量满时返回错误，不自动迁移到另一区域；同名属性改区域也会报错。

```rust
use bevy_gas::{
    AttributeId, AttributeIdManager, AttributeSet, AttributeSetError, ModifierOperation,
    ModifierSpec,
};

fn initialize_health(
    manager: &AttributeIdManager,
    health: AttributeId,
) -> Result<AttributeSet, AttributeSetError> {
    let mut attributes = AttributeSet::default();
    attributes.initialize_attribute(manager, health, 100.0, None)?;
    attributes.apply_instant_modifier(
        manager,
        &ModifierSpec::new(health, ModifierOperation::Add, -25.0),
    )?;
    Ok(attributes)
}
```

上例直接操作属性层，用于说明数值 API。常规战斗伤害通过 Effect/队列提交，才能保留统一顺序、效果概率、免疫和标签规则。

`initialize_attribute` 用于初始化，重复调用会清除该属性已有聚合器和修改器，不应把它当作通用回血接口。

### Base 与 Current

Base 是没有持续修饰时的基础值，Instant 修改直接改变它。Current 是将当前持续修饰与 Base 聚合后的结果。

例如攻击力 Base 为 100，Buff 添加 20，Current 为 120；Buff 到期移除后回到 100。瞬时受到 30 伤害则直接降低 Health 的 Base，不会因为“伤害效果已经结束”恢复。

| 读取 API | 特点 |
| --- | --- |
| `get_current_value(&mut self, manager, id)` | 必要时先重算，更新缓存，返回 `Result<Option<f32>, ...>` |
| `evaluate_attribute(&self, manager, id)` | 只读计算最新快照，不清脏位、不修改缓存 |
| `make_snapshot(&mut self, entity)` | 先重算脏属性，再捕获整组属性快照 |

`Ok(None)` 表示 ID 合法但该实体未初始化；非法 ID 则返回错误。业务需要区分这两个情形，不能把所有缺失都默认成 0。

### 默认聚合公式

不存在 Override 时：

```text
Current = ((Base + Σ Add) × (1 + Σ PercentAdd)) × Π Multiply
```

假设 Base 100，Add 10，两项 PercentAdd 分别 0.2、0.3，一项 Multiply 为 2，则结果为 `(100 + 10) × 1.5 × 2 = 330`。

| 操作 | 持续聚合 | 瞬时操作对 Base 的影响 |
| --- | --- | --- |
| Add | 先累加到基础值 | `base += value` |
| PercentAdd | 所有贡献先求和，再乘 `1 + sum` | `base *= 1 + value` |
| Multiply | 逐项连乘 | `base *= value` |
| Override | 有覆盖值时直接返回覆盖值 | `base = value` |

瞬时修改按定义顺序逐个执行。两个瞬时 PercentAdd(0.1) 产生 `1.1 × 1.1`，与两个持续百分比共同形成 `1 + 0.1 + 0.1` 不同。

当前聚合器只有一个 Override 槽，后写覆盖前写；撤销后写的来源不会恢复之前被覆盖的 Override。需要优先级覆盖栈时应在游戏/库机制中明确设计，不能依赖当前槽位自动回退。

### 脏位与来源移除

持续贡献通过 `ModifierSourceId` 关联来源，移除 Buff 时只撤销该来源贡献。游戏可使用 `ModifierSourceId::new(scope, slot, generation)` 为自己的长期修饰建立身份；游戏来源和效果内部来源在不同 domain，避免相同数字互相误删。游戏仍需避免自己分配重复身份。

每个 AttributeSet 维护 Hot/Cold dirty 位图。持续修饰添加/删除时只标记受影响属性；统一重算系统先筛选 `Changed<AttributeSet>`，再扫描脏位。聚合器采用按位置排序的稀疏 Vec，只为有持续贡献或自定义执行器的属性保留条目。

这减少了每 tick 重算所有角色全部属性的开销，同时允许需要即时值的 API 按需计算。

### 自定义聚合和执行后回调

`initialize_attribute` 的 executor 参数可设置 `fn(&Aggregator, f32) -> f32`。执行器也可能用于费用预演和只读求值，必须无副作用，不能依赖“本次被调用了几次”。

`AttributePostExecute` 由 `set_post_execute` 设置，签名包含属性集、管理器、属性 ID、旧 Current、新 Current。它在瞬时执行后调用，持续聚合重算和可支付性预演不会调用。可用于实现游戏自己的数值约束；它没有 Commands 参数，涉及死亡实体处理、事件或表现时应由额外系统接续。

GAS 没有自动规定 Health 上限、死亡规则或 Damage 属性结算协议。若游戏需要这些语义，明确放入业务层，并考虑成本预演不会执行 post-execute 副作用这一边界。

详见 [属性](../../bevy_gas/.docs/04-attributes.md) 和 [修改器与聚合](../../bevy_gas/.docs/05-modifiers-and-aggregator.md)。

## 11 Modifiers 与数值捕获

`Modifier::new(attribute_id, operation, magnitude)` 描述“改哪个属性、怎样改、幅度从哪里来”。`ModifierMagnitude` 有 `Flat(f32)` 和 `Calculated(Box<dyn ModifierMagnitudeCalculation>)`。模板的 LinearLevel 是游戏配置层实现的 Calculated，不是 GAS 内置的第三种枚举。

```rust
use bevy_gas::{AttributeId, ModifierEvaluationContext, ModifierMagnitudeCalculation};

struct AttackDamage {
    attack: AttributeId,
    coefficient: f32,
}

impl ModifierMagnitudeCalculation for AttackDamage {
    fn calculate(&self, context: &dyn ModifierEvaluationContext) -> f32 {
        let Ok(Some(attack)) = context.source_attribute(self.attack) else {
            return 0.0;
        };
        -attack.current() * self.coefficient
    }
}
```

将其放进 `ModifierMagnitude::Calculated(Box::new(AttackDamage { ... }))`，配合 Health 的 Add 操作即可构造伤害。该例主动选择“来源当前属性不可用时伤害为 0”；不同玩法可以采用自己的明确策略。

`ModifierEvaluationContext` 可以读取等级、source、target、instigator、causer、来源/目标当前属性、标签以及可选 source snapshot。live source 读取不会隐式回退到 snapshot，需要公式自己选择使用快照还是实时值。

### 捕获发生在什么时候

```text
共享 GameplayEffect 定义
  -> make_spec(context)
  -> 计算 modifier magnitude、duration、period
  -> GameplayEffectSpec 保存已求值结果
  -> 持续聚合或后续周期执行复用这些值
```

构造 Effect 定义时不会求值；形成 Spec 时求值一次；周期 pulse 不会每次重读攻击力。技能延迟任务保存 Effect 定义，真正应用该效果时才按对应上下文求值，而不是一概在技能激活时计算好未来所有伤害。

如果希望弹体命中仍使用发射瞬间攻击力，游戏应捕获并传播 source snapshot，公式明确读取它。只创建快照但公式仍调用 `source_attribute`，不会自动实现快照伤害。

属性修改幅度公式返回 NaN/Infinity 会在效果应用或属性修改入口被拒绝。定义和 Spec 构造本身不保证应用成功；自定义公式必须明确处理不存在的属性和目标。

## 12 Gameplay Effects

### 三类常用效果

| 定义 | 运行行为 | 常见用途 |
| --- | --- | --- |
| Instant | 顺序修改 Base，不创建持续记录 | 直接伤害、治疗、属性消耗 |
| DurationTicks，无周期 | 在寿命内保留聚合贡献，到期撤销 | Buff、Debuff、冷却标签 |
| Infinite，无周期 | 一直保留，直到明确移除 | 装备、被动状态 |
| Duration/Infinite，有正周期 | 保存活动记录，每次 pulse 按瞬时操作修改 Base | DoT、HoT、持续资源消耗 |

创建持续减速示例：

```rust
use bevy_gas::{
    AttributeId, EffectDurationTicks, EffectTags, GameplayEffect, GameplayTag, Modifier,
    ModifierMagnitude, ModifierOperation, StackingPolicy,
};
use std::sync::Arc;

fn make_slow(
    move_speed: AttributeId,
    asset_tag: GameplayTag,
    state_tag: GameplayTag,
) -> Arc<GameplayEffect> {
    Arc::new(GameplayEffect::new(
        vec![Modifier::new(
            move_speed,
            ModifierOperation::PercentAdd,
            ModifierMagnitude::Flat(-0.3),
        )],
        EffectDurationTicks::DurationTicks(ModifierMagnitude::Flat(60.0)),
        None,
        1.0,
        StackingPolicy::non_stacking(),
        EffectTags::new(vec![asset_tag], vec![state_tag]),
    ))
}
```

在默认 20 Hz 下持续 3 秒，撤销时删除自己的 -30% 聚合贡献和状态标签引用。non_stacking 表示每次应用独立记录，所以连续两次减速会产生两项 PercentAdd，不能把它理解成“重复施加自动忽略”。

### 应用 API 和上下文

独立同步调用：

```rust
use bevy::prelude::Entity;
use bevy_gas::{
    EffectPayload, EffectSystemParams, GameplayEffect, GameplayEffectApplicationError,
    apply_gameplay_effect,
};
use std::sync::Arc;

fn apply_hit_now(
    source: Entity,
    target: Entity,
    effect: &Arc<GameplayEffect>,
    params: &mut EffectSystemParams,
) -> Result<(), GameplayEffectApplicationError> {
    let payload = EffectPayload::new(source, None, 1);
    apply_gameplay_effect(target, effect, params, &payload)
}
```

这是同步入口，绕过公共请求顺序；普通运行时命中系统若要与技能请求保持 FIFO，应向 `GameplayExecutionQueue` 提交效果请求。

`EffectPayload::new(source, causer, level)` 默认 instigator 为 source；可以用 `with_instigator`、`with_source_snapshot` 补充。source 是提供属性/标签的游戏来源，instigator 是动作发起者，causer 可以是实际弹体，target 是本次受影响实体。

### 应用检查和执行顺序

效果准备大致按以下顺序进行：

1. 校验概率；小于 1 时先采样，失败即拒绝。
2. 检查来源和目标 application requirements。
3. 检查目标现有效果提供的应用免疫。
4. 求值 Modifier、时长和周期，形成 Spec。
5. 验证持续时长、组件、属性初始化、标签和数值合法性。
6. 计算待移除效果，决定瞬时执行、新建活动记录或合并叠层。
7. 验证计划后执行，处理数值/标签贡献，再按需收敛效果条件。

因此概率效果即使后来被标签、免疫或组件检查拒绝，也可能已经消耗一次 RNG。同步应用返回前会处理相关条件收敛，但不承诺撤销所有已经发生的副作用。

`prepare_gameplay_effect` 和 `execute_gameplay_effect_plan` 提供分阶段 API。Plan 应紧接着执行；执行阶段不会重新掷概率或重新做全部应用条件、免疫、叠层决策，不能将它长期缓存当作预约施法。

### 周期和到期边界

`EffectPeriodTicks::new(magnitude, execute_on_applied)` 配置周期。正周期效果的每次 pulse 都修改 Base，移除效果不会恢复已经扣除的生命。

- `execute_on_applied=true` 在新建正周期记录时立即 pulse，正常周期从后续 EffectTicks 推进。
- 该首次 pulse 在首次 ongoing 条件收敛之前；随后被抑制不代表首跳未发生。
- 叠层不会重复执行“应用时首跳”。
- duration 先于 period 推进；到期并被移除的记录不再 pulse。RemoveSingleStack 在仍有层时保留记录，同 tick 仍可能按剩余层数执行周期效果。
- `make_spec` 捕获的幅度被周期重复使用，不会自动动态追踪施法者攻击力。

底层浮点时长/周期转 tick 时正数向上取整，极大值饱和；非有限或非正数变为 0。DurationTicks(0) 被应用检查拒绝；周期 0 则按无周期持续聚合处理。模板配置对时长/周期执行更严格的正数及表示检查，避免依赖这一底层边界。

### 标签 免疫和抑制

`EffectTags::new(asset_tags, granted_tags)` 的两组标签含义不同：asset 标识效果类别，不自动进入角色容器；granted 在该效果贡献生效时授予目标。

| Builder | 行为 |
| --- | --- |
| `with_application_requirements(source, target)` | 申请时双方条件都需满足 |
| `with_ongoing_requirements(source, target)` | 活动期间双方条件都满足才贡献效果 |
| `with_removal_requirements(source, target)` | 任一非空侧条件满足则永久移除 |
| `with_granted_application_immunity(queries)` | 活跃且未抑制时，阻挡匹配来源与 incoming asset 条件的新效果 |
| `with_remove_effects_with_tags(tags)` | 本次执行先移除匹配的现有效果 |

ongoing 不满足时，效果进入 inhibited：撤回持续聚合贡献和 granted tags，停止提供免疫，暂停周期进度。有限 duration 仍继续减少。恢复条件后重新安装贡献，周期从原进度继续，不重新执行首跳。

标签改变可能抑制 A，A 撤回标签又影响 B，因此实现会按稳定的目标和效果顺序反复收敛到固定点。若发现重复状态形成不收敛循环，会移除参与循环的效果并记录错误。设计条件时避免互相授予和撤销同一组条件导致振荡。

`EffectRequirementDiagnostics` 默认关闭，开启后可读扫描、转换、循环和耗时指标。耗时仅用于观测，不参与玩法决策。

### 堆叠策略的每个维度

| 维度 | 选择 | 含义 |
| --- | --- | --- |
| 身份 | None | 每次独立记录 |
| 身份 | AggregateBySource | 同 Arc 定义、同目标、同 source 合并 |
| 身份 | AggregateByTarget | 同 Arc 定义、同目标合并，source 可以不同 |
| 幅度 | None / Linear | 保留原幅度 / 直接将每个 magnitude 乘层数 |
| 时长 | KeepExisting / RefreshOnSuccessfulStack | 保留寿命 / 成功叠加时用新求值时长刷新 |
| 周期 | KeepCurrentTick / ResetOnSuccessfulStack | 保留进度 / 成功叠加后进度归零 |
| 溢出 | RejectApplication / RefreshDuration | 拒绝 / 保持层数并进入成功路径 |
| 到期 | RemoveAllStacks / RemoveSingleStack | 全部移除 / 减一层并在仍有层时重置寿命 |

`stack_limit=0` 表示无业务层数上限。`linear_refreshing(type, limit)` 组合 Linear、刷新时长、重置周期、溢出拒绝、到期全移除。

有三个必须明确的实现细节：

1. Linear 对所有操作都直接乘幅度。Multiply(1.2) 两层变成 Multiply(2.4)，不是 1.44；Override 也乘层数。百分比叠加通常更适合 PercentAdd。
2. 加层保留原活动记录的 source，以及旧 Spec 的幅度和周期长度，不为每层保留独立快照。新 duration 可以刷新剩余寿命，但不替换整个旧 Spec。
3. 溢出 RefreshDuration 只允许保持层数走成功路径；是否真刷新仍由 duration policy 决定。配合 KeepExisting 时不会刷新寿命。

“重复命中只刷新、强度不增加”的玩法应明确组合 AggregateByTarget、limit 1、幅度 None、允许溢出成功和刷新时长，不能仅凭 non_stacking 或一个溢出枚举推断行为。

### 查询和移除

活动效果位于目标自己的 `ActiveGameplayEffects` 内，句柄包含目标、容器身份、槽位和 generation。同 World 的旧句柄不会因槽复用误命中新效果，但不能跨 World 使用。

`get_active_effects_on_target` 返回 Handle 列表；需要避免额外 Vec 分配时，可以从 `ActiveGameplayEffects::handles(target)` 迭代，再用 `get(handle)` 查看 Spec、source、层数、抑制与计时状态。`remove_active_effect(handle, params)` 返回是否真正移除；同 World 的陈旧句柄返回 false。

移除撤销该效果尚存在的持续贡献和标签引用，不回滚历史 Instant 或周期伤害。直接移除/替换 ActiveGameplayEffects 时，生命周期 hook 也会清理相关贡献。

当前按标签查询/移除效果的函数将 asset tags 和查询 tags 双方都扩展祖先，再取交集。因此 `Effect.Buff` 与 `Effect.Debuff` 可能因为共同祖先 `Effect` 命中。需要精确驱散时，应明确筛选目标效果的定义或原始 asset tag，再按 Handle 移除，不能把这类接口当作严格相等筛选。

详见 [效果模块](../../bevy_gas/.docs/06-gameplay-effects.md)、[应用计划](../../bevy_gas/src/gas/gameplay_effects/active_gameplay_effect/planning.rs) 和 [周期推进](../../bevy_gas/src/gas/gameplay_effects/active_gameplay_effect/ticking.rs)。

## 13 Gameplay Abilities 与 ASC

### 定义技能并授予角色

`GameplayAbility` 包含标签规则、成本、冷却、额外成本、启动任务和多实例策略。定义通过 Arc 共享，状态存放在角色 ASC 和活跃技能实体中。

常用 builder 为 `with_tags`、`with_cost`、`with_cooldown`、`with_additional_costs`、`with_startup_tasks`、`with_allow_multiple_instances`。列表 builder 替换整组列表，不是追加。

```rust
use bevy_gas::{
    AbilityGrantError, AbilitySpecHandle, AbilitySystemComponent, AbilityTaskDef,
    AbilityTaskOnFinishedDef, GameplayAbility, GameplayEffect,
};
use std::sync::Arc;

fn grant_strike(
    asc: &mut AbilitySystemComponent,
    cost: Arc<GameplayEffect>,
    cooldown: Arc<GameplayEffect>,
    damage: Arc<GameplayEffect>,
) -> Result<AbilitySpecHandle, AbilityGrantError> {
    let ability = GameplayAbility::default()
        .with_cost(cost)
        .with_cooldown(cooldown)
        .with_startup_tasks(vec![AbilityTaskDef::wait_ticks(
            5,
            AbilityTaskOnFinishedDef::Batch {
                actions: vec![
                    AbilityTaskOnFinishedDef::ApplyGameplayEffectToTargets { effect: damage },
                    AbilityTaskOnFinishedDef::EndAbility,
                ],
            },
        )]);
    asc.give_ability(Arc::new(ability), 1)
}
```

`give_ability` 返回该角色自己的 Handle；耗尽时返回 `AbilityGrantError`，不会回绕复用旧编号。`find_ability_spec` 查已授予规格，`clear_ability` 撤销授予，有活跃实例时拒绝。通过模板授予的技能优先使用成对的 `grant_ability`/`revoke_ability`，保持配置映射同步。

默认技能没有任务、成本或冷却，禁止多个并发实例。它被激活后会保持 Active，必须显式结束或取消；任务列表执行完毕也不会自动结束。

### 五组标签规则

| AbilityTags 字段 | 检查或修改什么 |
| --- | --- |
| ability_asset_tags | 标识技能类别，不授予角色 |
| cancel_abilities_with_tags | 启动时取消匹配类别的其他活跃技能 |
| block_abilities_with_tags | 当前技能活跃时阻止匹配类别的技能激活 |
| activation_required_tags | 来源角色必须拥有全部状态标签 |
| activation_blocked_tags | 来源角色具有任意一个标签则拒绝激活 |

阻止其他技能的计数存放在 ASC 内部，与 Actor 的 GameplayTagContainer 分开。不要把 ability asset tag 当作角色已经获得的状态。

完整角色应使用 Bundle。当前激活检查在来源缺少 GameplayTagContainer 时会跳过对应的角色标签和冷却标签判断，缺组件不能当作“所有受限技能都会安全拒绝”的保障。

### 从请求到启动的顺序

一次正式激活依次处理：

1. 验证激活链和 Handle，取得来源 ASC 与已授予规格。
2. 检查实例限制、技能阻止标签、来源 required/blocked tags 和冷却标签。
3. 检查外部成本，准备属性成本和冷却计划，预演可支付性。
4. 验证本技能要添加的阻止状态，再取消指定旧技能。
5. 创建活跃实例和 pending 状态，增加活跃计数与阻止计数。
6. 重新验证支付条件，暂扣外部资源，执行属性成本与冷却。
7. 支付成功后按定义顺序执行启动任务。

**激活自动 Commit。** 调用者不应在激活成功后再次调用 `commit_ability`，否则会再次支付。`commit_ability` 是单独提交成本与冷却、不开技能实例的独立 API。

成本和冷却施加在 source 身上。属性成本必须是 Instant、只含 Add，通常以负数表示消耗。可支付性按修改器顺序预演聚合结果，包括已有持续贡献；费用预演不会执行 post-execute 副作用。

冷却通过一个效果的 granted tags 阻止再次激活，而不是 ActiveGameplayAbility 自己保存一个冷却秒表。技能结束之后，冷却效果继续存活到自己的到期时间。

### 预检与正式激活

`can_activate_ability` 与 `AbilityActivationCheckParams<P>` 适合 UI 可用状态和 AI 决策，检查当前标签、冷却和支付能力。它是只读预测，不消耗概率随机数，也不完整检查授予 Handle、实例数量、Cost 是否 Instant 或效果应用要求。真实执行仍可能失败。

`try_activate_ability_by_handle` 是独立同步调用路径，不会加入公共 FIFO，也不会替调用者处理已经排队的其他请求。普通游戏生产系统优先入队，避免同步调用与队列等待中的逻辑产生不直观的顺序。

### 活跃技能 结束和取消

```text
Actor
└── ActiveGameplayAbility
    ├── AbilityTask
    └── AbilityTask
```

`ActiveAbilityHandle` 是活跃实例 Entity。`end_ability(source, active_handle, params)` 标记 Ending，`cancel_ability(...)` 标记 Cancelled；后续 Cleanup 撤销该实例的阻止计数、减少活跃数、删除实例与任务。

结束和取消都不会自动退款，不会移除此前已经施加的效果，也不撤回先前已经进入公共 FIFO 的请求。冷却、DoT、召唤物是否随技能结束移除，需要游戏明确设计。

通过 Commands 新建的实例在 flush 前不能直接作为普通 Query 结果读取，内部 `PendingActiveGameplayAbilities` 为当前激活批次提供临时可见性，使同步子技能可以正确取消父技能。这是运行时内部协调资源，不应拿来存储游戏状态。

直接移除或替换 ASC、活跃实例组件，以及销毁实体时，生命周期 Observer 也负责清理遗留记账。游戏常规结束流程仍应优先调用公开 end/cancel API。

### 背包 弹药等额外成本

`AdditionalCost::new(resource_id, amount)` 创建整数资源需求，ID 为 i32、amount 为正 u32；无须注册资源名称。GAS 不管理背包，游戏通过 `AdditionalCostProvider` 提供支付能力。

| Provider 部分 | 责任 |
| --- | --- |
| ReadOnly | 定义 UI/AI 只读检查所需的 SystemParam |
| Receipt | 拥有失败补偿所需的数据，不能借用 ECS，Drop 不自动退款 |
| check / check_readonly | 检查整批需求，不修改状态 |
| prepare | 重新验证整批并同步暂扣；失败时必须保持库存不变 |
| rollback | 后续属性成本或冷却执行失败时恢复已暂扣外部资源 |

重复项必须累计。例如库存只有 1 发弹药，成本两项各消耗 1，不能因为每项单独可支付而放行。Provider 应先检查总量、溢出、资源存在和规则，再一次完成扣减，不能把付款放到 deferred Commands 或 Observer，否则同 tick 多次激活可能重复花费最后一件物品。

在模板主程序共享装配位置选择：

```text
SimulationPlugin::new(settings).with_additional_costs::<InventoryCosts>()
```

同时初始化 Provider 需要的游戏组件和资源。直接调用 GAS 的 `AbilitySystemParams<InventoryCosts>`、预检参数和运行时插件必须使用相同 Provider。默认 `()` 遇到非空额外成本会拒绝，不会免费放行。

补偿范围是 Commit 期间暂扣的外部成本；成功支付之后的启动动作失败、未命中、取消或结束不自动退款。该协议也不保证把先前取消的技能、已执行属性变化等全部回滚，不能视作整个战斗事务。

完整实现参考 [inventory_bomb.rs](../../bevy_gas/examples/inventory_bomb.rs) 和 [扩展额外成本协议](../../bevy_gas/.docs/14-extending-the-system.md)。

## 14 Ability Tasks 与技能链

### 内置任务与完成动作

内置 TaskDef 只有两种：

| 定义 | 行为 |
| --- | --- |
| `AbilityTaskDef::instant(action)` | 启动期间立即处理动作，不创建等待实体 |
| `AbilityTaskDef::wait_ticks(ticks, action)` | 创建任务实体，由后续 AbilityTasks 阶段推进 |

| OnFinished 动作 | 用途 |
| --- | --- |
| None | 完成任务，不执行额外动作 |
| EndAbility | 结束父技能 |
| Batch | 按顺序处理多个动作，支持嵌套 |
| EmitEvent | 触发带 UniqueName ID 的 AbilityTaskEvent |
| ApplyGameplayEffectToTarget | 应用到捕获的主目标 |
| ApplyGameplayEffectToTargets | 按捕获顺序应用到全部目标 |
| ActivateAbility | 激活同来源角色已授予的另一个 Handle，传播目标和链上下文 |

不存在内置 WaitSeconds、WaitSignal、移动、播放动画或命中检测任务。需要这些能力时由游戏系统和现有事件/请求协作，或明确扩展原生任务机制。

### 时间线和 Batch

多个 WaitTicks 从同一激活起点并行等待。例如 `[wait(5, damage), wait(6, end)]` 是 tick 5 伤害、tick 6 结束，绝不是 tick 11 结束。

WaitTicks(0) 和 WaitTicks(1) 都在下一次被任务系统处理时完成。WaitTicks(0) 不是启动 Instant。正数 N 对应后续 N 次任务阶段推进。

Batch 深度优先、按顺序处理，遇到 EndAbility 会终止本技能后续动作遍历。前面已经生效的效果或已经入队的请求保留。

多个独立任务同 tick 完成时，任务系统按 Entity bits 排序。需要确定的“先伤害再结束”语义时，放进一个 Batch；不要依赖多个实体的偶然创建顺序。模板配置编译器已经把同 tick 动作合并成 Batch。

### 启动同步执行和运行时队列执行

同一个完成动作在两条路径中的执行时机不同：

| 路径 | ApplyEffect / ActivateAbility 的行为 |
| --- | --- |
| 启动 TaskDef::Instant | 当前激活内部同步执行 |
| 等待任务到期 | 产生公共 FIFO 请求，稍后由 resolver 处理 |
| 手动创建运行时 AbilityTask::instant | 由任务系统处理，按运行时路径入队 |

例如公共队列为 `[激活 A, 激活 B]`，A 的启动动作是 `[应用 X, 激活 C, 结束 A]`，实际顺序为：

```text
A 的成本和冷却
  -> X 完成
  -> C 的成本和冷却及整个 startup 完成
  -> A 继续并结束
  -> 公共队列继续处理 B
```

实现使用显式执行栈，避免递归激活占用调用栈。子技能结束只结束自身；如果子技能规则取消了父技能，返回父帧后会停止父技能剩余启动动作。

内部效果或子技能失败会记录错误，后续可继续执行，不会把已经成功的根激活统一回滚。根请求 Succeeded 不等于每一个内部动作都成功。需要关键阶段独立确认时，游戏应设计明确的消息或状态反馈。

### 任务事件

EmitEvent 使用 `Commands::trigger(AbilityTaskEvent)`，由 `On<AbilityTaskEvent>` Observer 处理，不使用 MessageReader 读取。事件携带自己的上下文和目标数据；Observer 执行时不要假设父技能仍然 Active。

Observer 通过 deferred Commands 执行。任务阶段触发的 Observer 若在 resolver 前入队，可在当前 tick 处理；resolver 启动期间触发的 Observer 通常在 resolver 返回后才入队，其请求进入下一 tick。需要严格同步的效果顺序，用明确的 Instant/Batch 动作表达。

### 链上下文与限制

`AbilityActivationContext::direct`、`input` 创建常见根上下文；子技能继承 instigator、causer、来源快照和目标，通过链记录追踪路径。

默认 `ABILITY_CHAIN_MAX_DEPTH=8`，表示从根出发最多 8 条后继边，即根加 8 个后继；同一路径重复 Handle 被视为循环。根链通常用 `queue.new_root_chain(handle)` 创建。跨多个技能形成连招时，Handle 必须来自同一个来源 ASC。

默认限制用于阻止无限激活链，不代替游戏连招条件、输入窗口、可取消阶段或动画状态机。

详见 [技能任务](../../bevy_gas/.docs/08-ability-tasks.md) 与 [激活启动栈](../../bevy_gas/src/gas/ability_system/activation/startup.rs)。

## 15 统一执行队列与反馈

### 一个 FIFO 排序两类操作

`GameplayExecutionQueue` 同时保存 ActivateAbility 和 ApplyGameplayEffect。这样“先加增益再施法”与“先施法再加增益”有明确的不同结果，不会因技能队列和效果队列各自推进而丢失跨类型顺序。

| 方法 | 用途 |
| --- | --- |
| `push_activation(source, targets, handle, context)` | 已捕获目标的技能请求 |
| `push_chained_activation(...)` | 从父上下文派生链式请求 |
| `push_application(target, effect, payload)` | 游戏命中、环境伤害等效果请求 |
| `push(request)` | 提交完整请求值 |
| `new_root_chain(handle)` | 建立根激活链 |

```rust
use bevy::prelude::Entity;
use bevy_gas::{
    EffectPayload, GameplayEffect, GameplayExecutionQueue, GameplayExecutionQueueError,
    GameplayExecutionRequestId,
};
use std::sync::Arc;

fn queue_hit(
    queue: &mut GameplayExecutionQueue,
    source: Entity,
    target: Entity,
    damage: Arc<GameplayEffect>,
) -> Result<GameplayExecutionRequestId, GameplayExecutionQueueError> {
    queue.push_application(target, damage, EffectPayload::new(source, None, 1))
}
```

入队返回 RequestId；请求 ID 耗尽或链式入队的无效链错误在这里返回，实际应用错误由 resolver 的结果 Message 报告。队列中的 effect 仍是共享定义，具体幅度在实际准备应用时求值。

resolver 完整 drain，不设“每 tick 只处理前 N 条”的截断。必须控制游戏自己的请求产生规模，性能问题应从真实场景 Profiling 定位，不能假定库会替游戏把超额伤害自动分摊到未来。

### 读取结算结果

```rust
use bevy::prelude::*;
use bevy_gas::{GameplayAbilitySystemSet, GameplayExecutionOutcome, GameplayExecutionResult};

fn read_execution_results(mut results: MessageReader<GameplayExecutionResult>) {
    for result in results.read() {
        match &result.outcome {
            GameplayExecutionOutcome::Succeeded => {}
            GameplayExecutionOutcome::Rejected(error) => {
                debug!("gameplay request rejected: {error}");
            }
            GameplayExecutionOutcome::Failed(error) => {
                error!("gameplay request failed: {error}");
            }
        }
    }
}

fn install_execution_reader(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        read_execution_results.after(GameplayAbilitySystemSet::GameplayResolve),
    );
}
```

每个由默认 resolver 实际消费的公共请求有一次结果，包含 request_id、source、主 target 和 outcome，保持 FIFO 顺序：

- Succeeded：主操作成功。
- Rejected：正常玩法规则拒绝，例如冷却、资源不足、免疫或实例限制。
- Failed：无效配置、组件/规格缺失、未知资源、缺 Provider、执行错误等。

同步调用和启动内部子动作没有独立公共 RequestId/Result。根技能成功不代表延迟伤害成功，也不代表技能已结束；等待任务后来入队的伤害会有自己的执行结果。

结果消费者若再入队，默认等下一 tick。结果属于 Bevy Message，应及时消费，不能把 Message 保留期当成长期战斗日志存储。需要日志时游戏自行记录稳定业务字段。

### 队列的使用纪律

同一队列 ID 单调增加，`clear()` 不回收 ID。清空只丢弃未消费请求，不发布结算结果。正常游戏逻辑不要绕过插件调用 `pop` 或添加第二个 resolver，否则会破坏命令适配器对反馈的关联。

独立队列实例的相同数字没有全局意义，不能将请求 ID 当成网络实体 ID 或存档身份。多个系统向同一个队列生产时，仍要明确系统顺序；可变资源借用只保证不同时写，不保证先后固定。

详见 [执行管线](../../bevy_gas/.docs/16-gameplay-execution.md)。

## 16 目标结果与游戏空间索敌

### GAS 只接收结果

`bevy_gas::gas::gameplay_targeting` 保存捕获数据：

| 类型 | 内容 |
| --- | --- |
| AbilityTargetHit | Entity、世界位置、可选法线 |
| AbilityTargetData | 查询原点和有序命中列表 |
| AbilityActivationTargets | 单 Entity，或非空 TargetData；首个命中是主目标 |
| AbilityActivationData | 本次 source、targets、activation context 的组合 |

`AbilityTargetData::new(origin, hits)` 不排序、不去重、不检查几何。`AbilityActivationTargets::acquired(data)` 拒绝空集合，但不校验距离、阵营或实体存活。重复 Entity 会按列表被重复处理，多目标伤害去重应由游戏显式完成。

目标数据属于激活值本身，与 context 分开。context 承担来源传播、快照、技能链等信息，不应另存一份可能与主目标不一致的目标列表。

```rust
use bevy::prelude::*;
use bevy_gas::{
    AbilityActivationContext, AbilityActivationTargets, AbilitySpecHandle, AbilityTargetData,
    AbilityTargetHit, GameplayExecutionQueue, GameplayExecutionRequestId,
};
use bevy_gas_template::application::GameResult;

fn queue_captured_cast(
    queue: &mut GameplayExecutionQueue,
    source: Entity,
    handle: AbilitySpecHandle,
    origin: Vec3,
    ordered_hits: Vec<AbilityTargetHit>,
) -> GameResult<GameplayExecutionRequestId> {
    let targets = AbilityActivationTargets::acquired(AbilityTargetData::new(origin, ordered_hits))?;
    let chain = queue.new_root_chain(handle);
    let context = AbilityActivationContext::input(source, chain);
    Ok(queue.push_activation(source, targets, handle, context)?)
}
```

游戏可用物理引擎、手动选择或自己的查询产生 hits。若技能在若干 tick 后生效，目标仍是这次捕获的集合；GAS 不会替游戏重新命中检测。

### 模板的索敌规则

模板 `gameplay::targeting` 提供 `TargetingDefinition`。规则由一个 selection 开始，之后依序过滤和排序：

```rust
use bevy_gas_template::gameplay::targeting::{
    TargetingDefinition, TargetingDefinitionError, TargetingOperation, TargetingSortOrder,
};

fn nearby_targets() -> Result<TargetingDefinition, TargetingDefinitionError> {
    TargetingDefinition::new(vec![
        TargetingOperation::SelectSphere { radius: 8.0 },
        TargetingOperation::FilterSource,
        TargetingOperation::RequireAttributeSet,
        TargetingOperation::SortByDistance {
            order: TargetingSortOrder::Ascending,
        },
        TargetingOperation::Limit { count: 3 },
    ])
}
```

| 操作 | 行为 |
| --- | --- |
| SelectSelf | 选择来源，不要求 Targetable |
| SelectExplicitEntity | 选择输入中的明确目标，仍要求有效逻辑位置和 Targetable |
| SelectSphere | 选取球内目标位置点 |
| SelectCone | 选取锥内目标位置点，角度为半角弧度 |
| FilterSource | 去掉来源自己 |
| FilterTags | 用 TagRequirements 过滤角色状态 |
| RequireAttributeSet | 要求存在属性集，不保证某个具体属性已初始化 |
| FilterDistance | 按输入原点限制距离 |
| SortByDistance | 按距离排序，同距离有稳定 Entity tie-break |
| Limit | 保留前 N 个，N 必须大于零 |

一个定义只能有一个 selection，且必须第一项。规则顺序影响结果，先 Limit 再 Sort 与先 Sort 再 Limit 不等价。半径和距离应为非负有限值，锥方向执行时必须有效，锥半角使用 `0..=PI`。

输入用 `TargetingInput::new(origin, direction)`，显式目标用 `with_explicit_target(entity)`。origin 应来自 `LogicalPosition`，而非表现 Transform。

### 索敌请求与即时查询

常用路线是 `AbilityCommand::targeted`，由命令适配器管理请求和结果关联。底层还可使用 `TargetingRequestQueue::push_request`，continuation 选择仅返回结果或提交技能激活。

索敌处理器写入 `TargetingRequestResult` Message，并通过 Commands 触发 `TargetingResultEvent` Observer。取得目标成功、提交 GAS 成功、GAS 执行成功，是三个不同阶段，UI 不应看到索敌成功就宣告技能释放成功。

直接调用 `acquire_targets(source, input, definition, params)` 时，要保证范围查询发生在空间同步之后，并且其后没有尚未同步的位置/成员变化。首次空间索引未就绪会返回错误；没有可选目标返回 NoTargetsFound，而不是成功的空结果。

### 动态 AABB 树如何工作

模板通过 `LogicalPosition` 和 `Targetable` 建立内部空间索引。首次全量构建，以后利用组件变化和移除信息增量同步。

内部树用连续 Vec 节点池、空闲槽和父子索引组织。叶节点保存目标点的 fat AABB，小幅移动仍在包围盒内时无需重插；越界才更新树。插入选择包围表面积增长较小的分支，旋转维持树高，分支包围盒覆盖子树。

查询分两步：先用 AABB 排除大量无关节点，再用实体当前 LogicalPosition 做真正的球距或锥角判断。fat AABB 只是加速结构，不扩大真实可命中距离。

非有限位置从索引排除。组件移除由移除追踪和 Discard Observer 共同处理，避免长时间没有 fixed tick 时遗漏通知。

候选结果按 Entity bits 建立稳定顺序，距离相等也以实体编号打破平局，因此不依赖树遍历形状。不同机器若独立创建实体得到不同编号，仅有排序仍不足以保证跨机一致；网络游戏需自己的身份和生成顺序协议。

当前查询是点目标几何筛选，不包含单位体积、墙体遮挡、弹道、碰撞接触或伤害判定。需要这些能力时在游戏/物理层完成，再交给 GAS 结算。

详见 [空间索敌专题](05-spatial-targeting.md) 与 [逻辑位置示例](../examples/logical_position_targeting.rs)。

## 17 输入命令与表现

### 逻辑动作绑定

`AbilityInputBindings<Action>` 将游戏自定义 enum 动作映射到当前角色 ASC 的 Handle：

```rust
use bevy_gas::AbilitySpecHandle;
use bevy_gas_template::gameplay::ability_input::AbilityInputBindings;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Primary,
    Skill(u8),
}

fn make_bindings(handle: AbilitySpecHandle) -> AbilityInputBindings<Action> {
    let mut bindings = AbilityInputBindings::default();
    bindings.bind(Action::Primary, handle);
    bindings.bind(Action::Skill(1), handle);
    bindings
}
```

绑定组件应和 ASC 挂在同实体。`resolve(&action, &asc)` 检查动作存在且 Handle 仍被授予，不检查冷却/成本，也无法替调用者验证传入的 ASC 是否属于正确实体。

`bind` 返回旧绑定，重绑保留动作原位置；`unbind`、`unbind_ability` 清理引用；`iter` 保持插入顺序。解绑不会取消活跃技能或撤销已入队请求。撤销技能成功后同步清理输入映射，替换 ASC 或复制角色时重建绑定。

组件只负责逻辑映射，键盘/鼠标/手柄、按下/按住/松开、UI 选中单位及施法策略由游戏实现。

### 帧输入如何进入 fixed tick

建议在 `PreUpdate.after(InputSystems)` 读取输入边沿一次，将意图与当时解析到的 Handle 保存进游戏队列，再在 fixed tick 消费。

一帧可能没有 fixed tick，也可能执行多个追赶 tick。直接在 FixedUpdate 多次读取同一帧 `just_pressed`，可能重复施法；只在某个固定 tick 读取又可能遗漏无 tick 帧输入。持久队列既保存短输入，也隔离渲染与模拟频率。

游戏还要明确意图捕获语义：按键时捕获的目标/Handle 是否保持；若技能栏在消费前重绑，旧命令是否仍使用旧 Handle。模板示例使用捕获时的值，避免延迟消费改变原意图。

### 游戏命令队列

`GameCommandQueue<C>` 是泛型 FIFO，可用于 Move、Attack、Build 或技能命令，自身不会执行命令。输入、AI、UI 都可以向同类队列生产，由游戏系统消费。

不同 C 拥有不同队列，彼此没有统一跨类型顺序；需要“移动后立刻施法”等整体业务顺序时，可用游戏自己的 enum 包含多种命令，再在一个消费者中按顺序转换。

`AbilityCommandPlugin` 为 `GameCommandQueue<AbilityCommand>` 提供专门适配：先消费整批，为 targeted 取得目标，再按原命令顺序提交 GAS。混合 direct/targeted 不会因为 direct 省去索敌而抢先执行。

两个 AbilityCommand 构造器的 origin 默认均为 Input。`direct` 只表示目标直接提供；若是 AI/规则直接触发，可用 `with_origin(AbilityCommandOrigin::Direct)` 标明来源原因。

`AbilityCommandResult` 含 command_id、source、可选 targeting_request_id、可选 execution_request_id 和 outcome。Outcome 分为 Succeeded、TargetingFailed、SubmissionFailed、Rejected、Failed。

```rust
use bevy::prelude::*;
use bevy_gas_template::gameplay::commands::{
    AbilityCommandOutcome, AbilityCommandResult, AbilityCommandSet,
};

fn read_command_results(mut results: MessageReader<AbilityCommandResult>) {
    for result in results.read() {
        if let AbilityCommandOutcome::Succeeded = &result.outcome {
            debug!("activation accepted for command {:?}", result.command_id);
        }
    }
}

fn install_command_reader(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        read_command_results.after(AbilityCommandSet::Results),
    );
}
```

Succeeded 表示根激活成功，不能直接代表动画结束、伤害命中或技能全部动作成功。生产发生在 Prepare 之前时，本 tick 完成命令提交和反馈；更晚的命令保留到下一 tick。

### 逻辑位置和显示位置

`LogicalPosition(Vec3)` 是游戏权威世界坐标，移动系统在 `GameplaySet::Movement` 更新它。它不要求 Transform，也不根据父子 Transform 层级推导坐标。

显示实体添加 `PositionPresentation` 并安装 `PositionPresentationPlugin`：默认 interpolated，在 FixedFirst 捕获旧位置，在 PostUpdate 且 Transform 传播前写入根实体的 Transform.translation。

```text
Fixed simulation: previous logical position -> current logical position
Render frame: interpolate(previous, current, fixed overstep fraction)
```

插值带来一个 fixed tick 的表现延迟，让低频逻辑运动显示得连续。`PositionPresentation::direct()` 直接复制位置；传送或服务器校正后调用 `reset()` 清除插值历史，避免从旧位置滑过去。

只同步 translation，不修改 rotation/scale。组件会要求 Transform，但 LogicalPosition 仍须游戏提供。带 ChildOf 的表现根实体被拒绝，因为 LogicalPosition 是世界坐标；视觉偏移、武器和模型可放在其子节点。

缺组件、非有限位置或错误层级会记录在 `last_error()`，保留原 Transform，不传播坏坐标。完整渲染应用还需正常 Transform 插件，DefaultPlugins 已包含。

详见 [输入与空间随机](06-input-and-spatial-random.md)、[命令与反馈](08-game-commands.md) 和 [多单位示例](../examples/multi_unit_commands.rs)。

## 18 随机数 名称池与确定性

### 两条独立随机流

GAS 根模块公开 `Random`，模板公开 `gameplay::randoms::SpatialRandom`。两者都可固定 seed，但用途独立：

| 资源 | 用途 | 初始化 |
| --- | --- | --- |
| `bevy_gas::Random` | 范围随机、效果应用概率 | SimulationPlugin 设置 GAS seed |
| `SpatialRandom` | Bevy 类型随机、形状内部/边界采样 | 游戏显式初始化，SimulationPlugin 不自动添加 |

SpatialRandom 常用 `from_seed`、`set_seed`、`from_rng::<T>()`、`sample_interior(&shape)`、`sample_boundary(&shape)`。采样结果是形状局部坐标，世界平移与旋转由游戏处理。

GAS Random 常用 `random_range`、`random_bool`，直接调用者必须传入合法非空范围和 `[0,1]` 内有限概率；效果应用入口会额外验证概率。默认 seed 为 123456，基础启动器的 `--seed` 只改 GAS Random。

给两类行为不同资源，可以避免“多采样一次出生位置”改变下一次暴击/效果概率。表现粒子若也需要随机，继续使用自己的流，不消费战斗随机。

### UniqueNamePool

名称池将字符串驻留为 `UniqueName(u32)`，相同名称复用 ID。哈希用于查找候选桶，冲突时仍比较完整字符串，不把哈希碰撞当成相同名称。空字符串保留索引 0。

`new_name` 返回 Result，`get_display_str` 还原显示字符串。名称池服务标签、属性和任务事件等运行时身份，整数额外资源 ID 不需要经过它。

名称 ID 依赖当前池的登记顺序，不是稳定存档身份。已有注册表和技能定义仍引用名称时，不要单独 `clear()`：清空会复用旧索引，而旧 Handle 没有 generation，可能指向新的名字。

### 确定性承诺的边界

当前设计通过固定 tick、稳定数组/列表顺序、明确 FIFO 和独立随机流减少不确定因素。复现仍要求：

1. 相同依赖版本、配置、随机 seed 和随机调用顺序。
2. 相同输入进入的 tick、目标顺序及业务系统顺序。
3. 一致的实体生成/删除顺序，或游戏自己提供跨机稳定排序。
4. 不依赖 HashMap 遍历、墙上时钟或隐藏全局随机状态决定 Gameplay。
5. 自定义公式、聚合器和 Provider 没有不确定副作用。

这不等于已经实现跨平台逐位浮点确定性、网络预测或回滚系统。若需要回放/联机，游戏还要定义稳定命令记录、实体映射、配置版本、状态快照与重演协议。库中的 dirty 位和来源快照服务数值求值，不构成完整 World 回滚方案。

## 19 常见玩法的实现方式

| 玩法 | 建议组合 | 容易遗漏的规则 |
| --- | --- | --- |
| 直接伤害/治疗 | Instant Effect + Health Add | 负值/正值约定、生命上限和死亡逻辑由游戏定义 |
| 蓝耗技能 | Instant Add 负值作为 cost | 激活自动支付，不再重复 Commit |
| 普通冷却 | Duration Effect + granted Cooldown 标签 | 技能结束不移除冷却 |
| 暂时攻击增益/减速 | Duration + 无周期 Modifier | 与已有百分比的聚合规则、重复施加策略 |
| 毒伤/持续治疗 | 正周期 Effect | 幅度一次捕获，到期移除先于周期执行 |
| 眩晕/沉默 | granted 状态标签 + 技能 blocked tags | 新标签阻止未来激活；是否取消当前技能需额外设计 |
| 无敌/伤害免疫 | 活动效果授予应用免疫规则 | 明确 incoming asset tags，避免宽泛祖先匹配 |
| 范围伤害 | 模板范围索敌 + 捕获多目标 + ToTargets | 是否去重、是否命中时重新查询、是否逐目标付费 |
| 弹体 | 游戏生成/移动/碰撞弹体，命中后入队 Effect | causer 是弹体；伤害读取发射快照还是命中时属性 |
| 装备属性 | Infinite 持续 Effect，保存 Handle | 卸装按 Handle 移除，避免误删其他装备同类贡献 |
| 背包道具施法 | AdditionalCost + 游戏 Provider | 整批累计、同步扣款、补偿范围 |
| 连招 | 子技能 ActivateAbility + 链上下文 | Handle 属于同 ASC，深度和循环限制 |
| 蓄力 | 游戏维护蓄力状态，释放时提交普通激活 | 当前模板示例在释放时支付，不是 GAS 延迟 Commit |
| 引导技能 | 游戏/任务驱动周期动作与明确取消 | 停止后的已入队伤害、已支付成本不会自动撤销 |

新增玩法时先确定三个时刻：何时捕获目标、何时读取数值、何时支付成本。之后再选择任务和效果组合，可以避免把所有时序都模糊地放在“释放技能时”。

例如蓄力技能可在按下时进入游戏 ChargeSlot，松开时提交 GAS，取消或超时只清理尚未提交的蓄力状态。现成 [charged_ability.rs](../examples/charged_ability.rs) 使用这一方案。若需求是按下即扣费、持续可中断、松开按蓄力时长产生不同结算，则应明确设计另一套游戏状态和支付规则。

## 20 调试 验证与扩展边界

### 按流程定位问题

| 现象 | 优先检查 |
| --- | --- |
| cargo run 只有空窗口 | 基础 GamePlugin 为空；若要 Laser，运行 configured_game |
| path 依赖找不到 | 是否存在同级 bevy_gas，Cargo.toml 相对路径是否正确 |
| 配置文件不存在 | 是否执行完整导表，部署路径是否还是编译机默认目录 |
| manifest/schema/摘要失败 | 生成 Rust 和二进制是否同批次，是否误手改产物 |
| 有属性 ID 仍无法造成伤害 | 目标是否初始化对应属性，是否有需要的组件 |
| 索敌无目标 | LogicalPosition、Targetable、过滤条件、输入原点和索引同步时机 |
| 命令成功但看不到伤害 | 成功只代表激活；检查等待 tick、内部动作错误和后续效果结果 |
| 技能一直 active | 是否有显式 EndAbility，任务是否被正确推进 |
| 技能一直无法再次释放 | 活跃实例、block tags、冷却 granted tags、成本或状态条件 |
| Buff 消失但生命未恢复 | Instant/周期改的是 Base，移除不会回滚历史伤害 |
| 连续叠层数值异常 | Linear 是 magnitude×层数；Multiply 并非按层求幂 |
| 同一按键重复激活 | 是否在多个 fixed tick 重复读取同帧输入边沿 |
| 画面目标移动但索敌仍在旧处 | 游戏是否只改 Transform，未更新 LogicalPosition |
| MissingProvider / 未知资源 | 是否装配匹配 Provider，以及 Provider 是否识别该 i32 ID |
| MissingTargetingResult / MissingExecutionResult | 是否缺插件、改坏阶段顺序、手动清空/消费队列 |

启用日志时可在 PowerShell 设置 `$env:RUST_LOG = "info"`。GAS 结果适合精确关联请求，EffectRequirementDiagnostics 适合诊断条件收敛。不要用字符串日志替代需要程序判断的结构化结果。

### 推荐验证流程

基础检查在模板仓库执行：

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

修改配置、Schema 或生成模板后，先完整导表。保留示例时再执行：

```powershell
pwsh -NoProfile -File config/export.ps1
cargo run --bin gas-config -- validate-config assets/config
cargo run --bin gas-config -- inspect assets/config 1001 5
cargo test --all-features --examples
cargo test --no-default-features --examples
cargo run --example configured_game -- --headless
```

模板集成测试按 `tests/config_test.rs` 和 `tests/gameplay_test.rs` 组织；示例专用测试位于 examples 中，普通 cargo test 不代替显式的 `--examples` 测试。删除整个 examples 后，跳过对应命令。

修改 GAS 库时在 GAS 仓库执行自己的 fmt、全目标全 feature Clippy、test、build，再回模板完成相应回归。文档中的命令是开发检查清单，不表示每次只改文字也需要重跑所有构建。

### 什么扩展放在哪一层

| 需求 | 修改位置 |
| --- | --- |
| 增加技能数据、调整伤害和等待时间 | Excel，重新导表 |
| 新增配置字段或表 | Schema/表头、生成模板按需调整、配置校验与编译映射 |
| 新伤害公式 | 游戏实现 ModifierMagnitudeCalculation；需要可配置时加编译映射 |
| 输入、技能栏、AI、阵营和命中规则 | `src/game/`，复用 gameplay 模块 |
| 目标过滤、范围采样、移动和表现 | 模板 gameplay 或游戏自己的模块 |
| 自定义资源成本 | 游戏 AdditionalCostProvider |
| 新原生 Task 或 ModifierOperation | GAS 库机制及测试，再更新模板映射 |
| 存档、联网、预测、回滚、热更新 | 游戏自己的协议和系统，不能假定模板已经提供 |

当前没有通用第三方 AbilityTask 动态注册协议，也没有独立 GameplayCue/VFX/动画系统。表现可以订阅任务事件、命令/执行结果或角色状态；如何播放动画、音效、镜头和粒子由游戏实现。

容量由 `GameplayAbilitySystemSettings` 的编译期常量决定：属性 256、Hot 32、Cold 224、标签 512、技能链最大后继深度 8。修改容量会改变数组/位图布局，应统一客户端和服务端配置并重新构建验证，不是运行时改 Resource 即可生效。

### 进一步阅读与源码索引

| 主题 | 文档和入口 |
| --- | --- |
| 模板开始新游戏 | [起步指南](09-starting-a-game.md)、[GamePlugin](../src/game/plugin.rs) |
| 共享模拟 | [插件组合](07-architecture-and-composition.md)、[SimulationPlugin](../src/application/simulation/plugin.rs) |
| 配置所有权与制作 | [配置所有权](01-configuration.md)、[Excel 规则](03-gas-configuration.md)、[配置门面](../src/config.rs) |
| Luban 工具维护 | [工具链](02-luban-toolchain.md)、[导表入口](../config/export.ps1) |
| 依赖和模板维护 | [依赖指南](04-template-and-dependencies.md) |
| 输入 命令 空间 | [输入](06-input-and-spatial-random.md)、[命令](08-game-commands.md)、[索敌](05-spatial-targeting.md) |
| GAS 公共 API | [总览](../../bevy_gas/.docs/01-overview.md)、[速查表](../../bevy_gas/.docs/11-api-quick-reference.md)、[领域门面](../../bevy_gas/src/gas.rs) |
| GAS 插件和调度 | [插件生命周期](../../bevy_gas/.docs/02-plugins-and-lifecycle.md)、[runtime.rs](../../bevy_gas/src/gas/runtime_plugin/runtime.rs) |
| 标签 属性 修改器 | [标签](../../bevy_gas/.docs/03-gameplay-tags.md)、[属性](../../bevy_gas/.docs/04-attributes.md)、[聚合](../../bevy_gas/.docs/05-modifiers-and-aggregator.md) |
| 效果 技能 任务 ASC | [效果](../../bevy_gas/.docs/06-gameplay-effects.md)、[技能](../../bevy_gas/.docs/07-gameplay-abilities.md)、[任务](../../bevy_gas/.docs/08-ability-tasks.md)、[ASC](../../bevy_gas/.docs/09-ability-system-component.md) |
| 目标与队列 | [目标契约](../../bevy_gas/.docs/15-gameplay-targeting.md)、[执行管线](../../bevy_gas/.docs/16-gameplay-execution.md) |
| 自定义和配置接入 | [扩展机制](../../bevy_gas/.docs/14-extending-the-system.md)、[外部配置](../../bevy_gas/.docs/19-external-configuration.md) |
| 测试和源码维护 | [测试规范](../../bevy_gas/.docs/13-testing-guide.md)、[源码布局](../../bevy_gas/.docs/17-source-layout-and-maintenance.md) |

跨仓库链接使用默认同级目录布局。若调整 GAS 所在位置，同时更新依赖路径和相关文档链接。
