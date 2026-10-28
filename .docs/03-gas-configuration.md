# 03 — Excel 技能配置与 GAS 接入

## 范围与目录

支持标签、属性、效果、修改器、技能、动作时间线和目标规则七张表；额外消耗作为技能表的内联结构列表维护。
配置使用固定 Luban 5.0.0 的 `rust-bin + bin + --strict`，经安全读取和业务编译后形成
`GameplayCatalog` Resource。配置接入位于游戏 `bevy_gas_template` 包的 `config` 模块，
运行时接口和生成代码始终参与游戏编译；模板从自己的配置模块加载数据，只使用依赖库的 GAS 公共 API。
当前通过同级本地 path 使用 GAS；本地库已移除配置层，游戏独立维护全部配置。目录约定与可选依赖方式见 [04 — 模板使用与依赖维护](04-template-and-dependencies.md)。
默认开启的 `config-validation` feature 启用配置包校验、完整业务校验和离线工具；需要最小构建时用
`--no-default-features` 关闭。仓库只维护根 `Cargo.toml`。

| 路径                           | 职责                                                                   |
| ------------------------------ | ---------------------------------------------------------------------- |
| `config/tables/gas.*.xlsx`     | 策划维护的数据、Luban 字段类型与说明                                   |
| `config/defines/gas.xml`       | 七张 GAS 表的登记、普通枚举与内联额外消耗 Bean 定义                                        |
| `config/defines/builtin.xml`   | Luban 内置定义                                                         |
| `config/templates/rust-bin/`   | 项目维护的安全 Rust 模板                                               |
| `src/config.rs`、`src/config/` | 配置门面，以及解码、校验、GAS 编译、加载和包校验实现                   |
| `src/config/decoding/`         | 仅使用标准库的 Safe Rust、Result 二进制解码实现                        |
| `config/generated/`            | 自动生成的 `mod.rs`、`gas.rs` 等 Rust 模块，包含 DTO、表索引与结构描述 |
| `assets/config/`                  | 导出配置包，包括七张 GAS 表的 bytes 与 manifest.json；Git 忽略         |
| `src/bin/gas_config.rs`        | 启用 `config-validation` 后可用的配置预览与导表校验 CLI                |
| `src/gameplay/`               | 可复用的位置、空间索敌、输入、随机与命令能力                            |
| `src/application/`、`src/game/` | 共享模拟装配与自己的游戏业务入口                                      |
| `examples/configured_game/`   | 可删除的配置演示、窗口与无窗口验收及其测试                              |
| `tools/luban/.cache/export/`   | 每次导表的临时单包项目、验证产物与发布备份                             |

导表产物包含上述七张 GAS 表的七个 `.bytes` 文件和 `manifest.json`。
默认运行时还需部署匹配的 `manifest.json`；关闭 feature 的最小运行时只需部署七个 `.bytes` 文件。
`src/config.rs` 通过外部路径加载 `config/generated/mod.rs`，公开为
`bevy_gas_template::config::generated`；生成目录不维护独立 Cargo 包。

加载与预览分属 `loading` 和 `inspection`：前者负责读包解码，后者仅在 `config-validation`
启用时执行完整校验和文本报告。`ConfigError`、`ConfigErrorKind`、`ConfigLocation`
由配置领域共用的 `error` 模块拥有。编译器按准备、注册、效果、目标、技能时间线拆分：
`PreparedTables` 借用生成行，统一解析实际使用的动作、幅度参数和内联额外消耗，按父表 ID
列表建立技能动作、效果修改器的有序索引，并保留额外消耗列表的原始顺序。编译、完整校验
和预览共用这一视图，不重复扫描和解释同一关系。
生成 DTO 与运行时类型仍然分离，feature 行为保持不变；本次额外消耗内联改变配置二进制结构。
目标编译器构造本包 `gameplay::targeting::TargetingDefinition`，技能和效果仍构造 `bevy_gas` 类型。
两类定义分别由 CompiledAbility 持有，GAS 技能定义不反向包含游戏索敌规则。文件归属见
[01 — 配置所有权与扩展](./01-configuration.md)。

## Feature 边界

`default = ["config-validation", "windowed"]`。两个 feature 独立：`config-validation` 控制配置校验与工具，
`windowed` 控制 Bevy 窗口和渲染依赖。`--no-default-features` 同时关闭两者；无窗口程序需要完整校验时，
使用 `--no-default-features --features config-validation`；窗口程序需要关闭校验时，使用
`--no-default-features --features windowed`。`config-validation` 不控制配置读取、编译和授予能力，
也不控制生成模块的可见性。

| 能力                                                                                                                            | 默认构建                                           | `--no-default-features` 最小构建     |
| ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- | ------------------------------------ |
| 生成 DTO、`load_tables`、`read_package`、`compile_catalog`、技能授予与撤销                                                      | 可用                                               | 可用                                 |
| manifest、schema、文件与整包摘要校验                                                                                            | 必须通过校验                                       | 不编译，不读取清单                   |
| 文件与解码大小限制，非法枚举、重复主键及二进制合法性                                                                            | 始终检查                                           | 始终检查                             |
| 构建所需引用、运行时注册表容量与状态、基本数值合法性                                                                            | 始终检查                                           | 始终检查                             |
| 命名规范、未使用字段、跨表业务约束、时间线、成本与冷却策略                                                                      | `compile_catalog` 在注册前对准备结果执行完整校验   | 跳过完整业务校验                     |
| `package_schema_hash`、`MAX_MANIFEST_BYTES`、`validate_tables`、`describe_ability*`、`write_package_manifest`、`gas-config` CLI | 可用                                               | 不编译                               |
| 示例窗口与无窗口入口 | 均可用，并执行包校验和完整业务校验 | 无窗口入口可用，直接读取表数据；窗口入口返回 feature 未启用错误 |

默认运行时按生成的 `TABLE_FILES` 读取 `.bytes`，并校验同次导出的 `manifest.json`；最小构建不要求
`manifest.json`，已有清单即使无效也会忽略。I/O、分配边界、解码和必要构建错误仍返回 `Result`。
发布前的导表流程始终启用 `config-validation`，完成清单、结构、摘要与业务校验后才发布；运行时加载的
正是这批已验证的表数据。

基本数值检查包含正数最大等级、有限且位于 `[0, 1]` 的应用概率、实际使用的有限幅度参数，
以及正数持续时间/周期和转换为运行时 f32 tick 时的无损性。使用的球形/锥形半径与最大距离
也必须具有有限平方。最大等级 100、tick 上限 1,000,000 等制作限制属于完整业务校验。

在 Zed 中打开仓库根目录时，`src/config/` 和 `config/generated/` 已属于默认 Cargo 模块图，
rust-analyzer 无需额外启用 feature 即可分析运行时接口的定义与引用。
本项目 `.zed/settings.json` 已让 rust-analyzer 启用 `config-validation`，可同时分析完整校验与离线工具。

## 日常使用

当前导表支持 Windows x64；首次运行配置演示先准备同级 GAS 源码，再在新仓库根目录依次运行。基础 `cargo run` 不加载表格，无需完成这些导表步骤：

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run --example configured_game
```

预览配置或执行无窗口验收：

```powershell
cargo run --bin gas-config -- inspect assets/config 1001 3
cargo run --example configured_game -- --headless
```

首次导表需要已安装的 Rust 工具链及依赖缓存。导表使用锁文件和离线构建；
缺少依赖时先执行 `cargo fetch --locked` 准备依赖，再重新导表。

导表入口先在独立暂存目录生成代码和数据，仅提取 Rust 模块作为待发布代码。
脚本复制游戏 `src/`、`Cargo.toml`、`Cargo.lock`、可选 `examples/` 与 `tests/` 和候选生成模块，
保留同一 GAS 依赖，在隔离的 `target/config-export/` 构建启用 `config-validation` 的临时单包项目。
根游戏 Cargo.toml 保持相对 path；暂存清单将其转为绝对路径，继续使用同一份本地 GAS 源码。
如果以后主动选择 Git 依赖，则保留其 URL 与 rev。
新 CLI 生成配置包清单，再实际读取整个包并在
无窗口 Bevy App 中编译 GAS 定义。
全部成功后才发布生成代码和数据目录；发布失败尝试恢复上一份目录。
GAS 转换或 Rust 编译失败也会使导表返回非零退出码。
发布过程通过排他锁防止并发导表，逐文件核对发布前后摘要，并在失败时尝试回滚。
两个目录的替换不是对并发读取者的瞬时切换，导表期间不要启动配置加载。

不要将 Luban MCP 的原始 generate 输出直接作为可发布包。MCP 的 get_schema 和 validate
适合查询表结构、定位 Excel 问题；完整发布入口始终为 export.ps1。
Luban 的结构校验和 Rust 的玩法校验是连续两层，不能互相替代。

## Excel 表与引用

| 表                  | 主键 | 内容                                                  |
| ------------------- | ---- | ----------------------------------------------------- |
| gas.TbTag           | name | 完整标签名及说明                                      |
| gas.TbAttribute     | name | 属性名、Hot/Cold 区域及说明                           |
| gas.TbEffect        | id   | 持续、周期、概率、asset/granted 标签、modifier_ids    |
| gas.TbModifier      | id   | 属性、操作、Flat/LinearLevel 参数                    |
| gas.TbAbility       | id   | 等级、标签、消耗、冷却、目标规则、实例策略、task_ids、additional_costs 内联列表 |
| gas.TbAbilityTask   | id   | at_tick、动作、目标范围、effect_id                   |
| gas.TbTargeting     | id   | 选择、标签/距离过滤、排序与数量上限                   |

技能动作时间线的源文件为 `config/tables/gas.ability_task.xlsx`，在 XML 中登记为 `gas.TbAbilityTask`。
生成的 `config::generated::gas::AbilityTask` 表示一行配置，每行描述一个动作，由编译器按 tick 分组并合并为任务定义。
它与运行时的 `AbilityTask` Component 分属配置数据和 ECS 任务状态两个层次。

技能通过 `task_ids` 引用动作，效果通过 `modifier_ids` 引用修改器。这两个字段仍是分号
分隔的有序 ID 列表，留空表示没有对应项；子表不保存父 ID 或 `order`。同一父列表内禁止
重复 ID，并检查每个引用存在；不同技能或效果可以引用同一行。共享的是配置定义，每次
激活或应用仍创建自己的运行状态。改变单个父定义时，应新建子行并调整该父列表。

技能 `additional_costs` 则是内联 `gas.AbilityAdditionalCost` Bean 列表，每项包含整数资源 ID resource
和 amount，没有独立成本行 ID，也不引用额外消耗子表。列表顺序及重复项原样保留，各技能拥有自己的
配置值；不会因为两个技能填写相同内容而建立可联动修改的共享成本定义。

未引用的动作和修改器仍可保留为配置素材；完整业务校验检查其字段、引用与数值合法性。
依赖父技能等级、成本用途和结束动作位置的约束按实际引用关系检查。task_ids/modifier_ids
的引用与重复 ID 检查在最小构建中仍执行；内联成本按每个技能分别检查同资源累计数量。

表名、主键、输入文件、枚举和内联 `AbilityAdditionalCost` Bean 统一维护在 `config/defines/gas.xml`。
表字段仍从数据 Excel 表头读取；因此修改技能数值只改数据表，新增字段修改相应表头，
新增表或枚举修改 XML 定义。新增数据文件的路径相对于 `config/tables/`。
无需额外维护定义 Excel；未独立定义的行 Bean 由 Luban 根据表头生成。

第一行 `##var` 为字段名，`##type` 为类型，`##` 为说明；数据行 A 列留空。
普通整数、浮点和布尔值使用 Excel 原生值。可空字段留空，不填字符串 null；
集合与内联 Bean 使用各自表头和 Schema 指定的结构，不将自定义字符串解析规则混入配置编译器。

示例类型：

- 必须存在的效果引用：`int#ref=gas.TbEffect`。
- 可选效果引用：`int?#ref=gas.TbEffect`。
- 标签列表：`(list#sep=;),(string#ref=gas.TbTag)`。
- 技能动作列表：`(list#sep=;),(int#ref=gas.TbAbilityTask)`。
- 技能额外消耗列表：`(list#sep=;),gas.AbilityAdditionalCost`；每个 Bean 内部按逗号分隔 resource、amount，详见下文。
- 效果修改器列表：`(list#sep=;),(int#ref=gas.TbModifier)`。
- 属性引用：`string#ref=gas.TbAttribute`。

标签与属性表是配置的权威名单。效果和技能引用的父标签也需在 TbTag 明确登记，
否则 ref 校验会拒绝；运行时登记子标签时会自动建立祖先。
容量统计包含全部祖先，当前上限为 512 个标签、32 个 Hot 属性和 224 个 Cold 属性。
TbAttribute 定义属性身份与存储区域，不定义角色当前数值；角色创建时仍需初始化属性。

SelectionKind 使用 `SelfTarget`，Excel 别名可以填写 Self，避免 Rust 的 Self 关键字冲突。

## 数值、时间与动作

### 修改器操作

`gas.Modifier.operation` 支持六种基础运算。XML 配置枚举通过编译器显式映射到
`bevy_gas::ModifierOperation`；名称和编号使用 UE 的六种操作。

| 配置值 | 编码 | 持续效果的默认聚合阶段 |
| --- | --- | --- |
| `AddBase` | 0 | 先累加到 Base |
| `MultiplyAdditive` | 1 | 乘以 `1 + Σ(value - 1)` |
| `DivideAdditive` | 2 | 除以 `1 + Σ(value - 1)` |
| `Override` | 3 | 使用当前存储顺序中的第一个覆盖值 |
| `MultiplyCompound` | 4 | 逐项连乘 |
| `AddFinal` | 5 | 所有乘除完成后累加 |

`MultiplyAdditive` 的 `1.5` 表示 +50%，`0.7` 表示 -30%；
`MultiplyCompound` 的 `1.5` 表示乘 1.5，`DivideAdditive` 的 `2` 表示单项除以 2。
两个持续 `DivideAdditive(2)` 合并为除以 3；两个瞬时操作依次执行，则除以 4。
默认执行器在合并后的除数满足 `abs(divisor) <= 1e-8` 时使用中性除数 1；瞬时除数使用相同检查。
持续聚合先合并全部原始值，再检查最终除数；
不会逐项丢弃数值为零的贡献。自定义 executor 负责自己的除零与数值边界策略。
瞬时 `AddFinal` 与 `AddBase` 都是按定义位置直接增加 Base，不会延迟到整个效果末尾。
技能成本继续仅允许 `AddBase`，不会因为瞬时 `AddFinal` 也是加法而放宽。

迁移旧配置时，将 `Add` 改为 `AddBase`；将 `PercentAdd` 改为 `MultiplyAdditive`，
Flat 的 `base` 加 1，LinearLevel 的 `base` 加 1、`per_level` 不变；自定义幅度计算器同样返回倍率。
将 `Multiply` 改为 `MultiplyCompound`，幅度不变。数值配置必须把旧 Multiply 的编号 2 改为 4，
把旧 DivideAdditive 的编号 4 改为 2；Override=3、AddFinal=5 不变。
本仓库示例表已迁移，旧二进制不能与新代码混用。修改后必须运行
`pwsh -NoProfile -File config/export.ps1`，同时生成匹配的 Rust 类型、二进制和 schema 清单，
生成代码不得手工编辑。
游戏自定义已有操作的聚合方式可以通过 executor 与 `Aggregator::modifiers(operation)` 只读迭代器实现，
无需新增配置操作或修改 GAS；详见 [自定义聚合](10-template-usage-and-gas-internals.md#自定义聚合和执行后回调)。

### 幅度与时间校验

以下规则描述配置制作约定，由启用 `config-validation` 的完整业务校验执行；
最小构建仍检查构建定义所需的基本数值与引用合法性，但不重复执行全部策略检查。

Flat 使用 base，per_level 必须为零。LinearLevel 使用
`base + per_level * (level - 1)`，等级从 1 开始。
编译器、预览与运行时计算器共用求值规则；完整业务校验检查允许等级范围内的结果
可表示为有限 f32。最小构建检查实际使用的 base 与 LinearLevel 的 per_level 有限，
不检查 Flat 未使用的 per_level，也不预演全部等级。运行时以 f64 求值后转换为 f32，
超出范围的结果保留为非有限值，由 GAS 返回 `GameplayEffectApplicationError::NonFiniteModifier`，
错误包含目标实体和属性 ID；不再把无效公式替换为合法的零幅度。
GAS 在应用该效果的修改器之前检查幅度，失败时不会部分应用前面的合法修改器。
适配器授予技能时检查等级；独立使用导出的 Effect 定义时，调用者仍须遵循其等级契约，
即使传入超出制作范围的等级，运行时数值检查也不会因关闭 `config-validation` 而消失。

修改器按所属效果 `modifier_ids` 的列表顺序构建，不按修改器 ID 或 Excel 行位置排序。
同一个修改器可用于多个效果；成本效果仍对其引用的修改器集合执行成本约束。

首版效果统一 non_stacking。Instant 不允许周期或保留 granted tags；
DurationTicks 必须提供正整数持续时间，Infinite 不填写 duration_ticks。
period_ticks 留空表示无周期，有值时必须为正；execute_on_applied 只用于周期效果。

成本必须 Instant、AddBase、概率 1、非周期，每个属性只能有一条修改器，且全等级成本为负。
不消耗属性资源使用空 cost_effect_id。背包等额外消耗直接填写技能的 `additional_costs`
内联列表；编译器按列表顺序构造 GAS `AdditionalCost`，见下方技能内联额外消耗。
`cost_effect_id` 只表示属性 Effect 成本，不能把物品 ID 填入该列；两类成本可以同时使用。
技能表中两个成本字段相邻，列顺序为 `cost_effect_id`、`additional_costs`、`cooldown_effect_id`。

冷却必须正持续时间、非空 granted tags、概率 1、
无周期且无属性修改器。冷却检查使用 granted tags，不使用 asset tags。

每个技能先按 `task_ids` 解析动作，再按 `at_tick` 升序排列；同 tick 保持该列表中的先后顺序。
同 tick 动作编译为一个 `AbilityTaskOnFinishedDef::Batch`；每个正 tick 分组只创建一个等待任务。
at_tick=0 使用 Instant；正值使用 WaitTicks，所有时间都相对于同一次激活，并行计时，
不会因列表位置变成依次累计等待。时间只承诺 tick，秒数取决于游戏设置的 `Time<Fixed>`。
公开预览报告保留 `order` 字段，其含义为对应父 ID 列表中的零基位置；时间线排序后也保留
该原始位置，不再读取子表中的 order 列。

当前配置层只支持有限技能：每个技能必须显式配置恰好一条 EndAbility，且为最后一个有序动作。
即时技能把 EndAbility 放在 at_tick=0 的最后；需要等待的技能把它放在实际结束 tick。
底层运行时 API 仍支持由外部生命周期逻辑结束的技能，任务执行完毕不会自动结束。

Batch 按定义顺序处理，遇到第一个 EndAbility 后停止。at_tick=0 的 Instant 会在激活阶段
逐动作同步应用效果，因此 `[ApplyEffect, EndAbility]` 先尝试结算效果，再结束技能。
正 tick 的 WaitTicks 仍在完成时把效果写入公共 FIFO；后续 EndAbility 不撤回已入队效果。
两条路径都不提供事务回滚，不会让普通 sibling WaitTicks 自动变成串行任务，也不会让
EmitEvent Observer 在 startup resolver 内同步回写。完整边界见 [GAS 依赖库的技能任务说明](../../bevy_gas/.docs/08-ability-tasks.md)。

更早版本的 Ability 表中 `activation_effect_ids` 和 `end_on_activation` 列已删除。原即时效果
改为 TbAbilityTask 中 at_tick=0、kind=ApplyEffect、target_scope=AllCaptured 的动作，并在
技能 `task_ids` 中排在原即时动作之前；原结束标记为 true 时，追加同一 tick 的最后一条
EndAbility。原结束标记为 false 时，保留时间线中原有的 EndAbility，并引用对应动作 ID。

ApplyEffect 的 Primary/AllCaptured 分别使用激活时捕获的主目标/全部目标。
命中时重新抓取、独立 Self 动作、地面点空目标、弹体和跨技能配置动作尚未开放。
真实弹体可由游戏层通过现有 EmitEvent、游戏索敌和 GAS 统一效果队列扩展。
当前点索敌不代表碰撞命中；空间与未来命中规则的职责见 [05 — 空间索敌](05-spatial-targeting.md)。

## 技能内联额外消耗

在 `config/tables/gas.ability.xlsx` 的 `additional_costs` 字段直接填写有类型的成本列表。
元素是 XML Schema 定义的 `gas.AbilityAdditionalCost` Bean，没有独立成本行 id；不再维护独立成本表、
成本行引用或占位行。空列表表示没有额外消耗。

技能表当前 F 列的 `##var` 为 `additional_costs`，`##type` 为
`(list#sep=;),gas.AbilityAdditionalCost`。XML Bean 使用 `valueType="1"`、`sep=","`：
列表用英文分号 `;` 分项，每项按英文逗号 `,` 分成 resource、amount 两个字段。
这是 Luban 的容器与 Bean 组合 sep 规则，由生成器读取为 `Vec<AbilityAdditionalCost>`。
参见 [Luban 官方 sep 与流式填法](https://www.datable.cn/docs/excel/vertical-and-sep)。

| 单元格内容 | 含义 |
| --- | --- |
| 留空 | 空列表，无额外消耗 |
| `5001,1` | 消耗 1 个资源 5001（示例 Bomb） |
| `5001,1;5002,3` | 依次定义 Bomb 1、Ammo 3 两项成本，示例 Ammo ID 为 5002 |
| `5001,1;5001,1` | 保留两项资源 5001 的成本，整批需要 2 个 |

每个非空列表项必须完整提供整数资源 ID 和数量。XML 的 resource 类型为 `int`，生成为 Rust
`i32`，允许完整范围 `-2147483648..=2147483647`，不额外禁止负数或 0。ID 是否对应游戏中的
有效资源由 Provider 判断。amount 仍为 `long#range=[1,4294967295]`；源表数量校验与 Rust
编译器的必需检查同时保留，不能只依赖 Excel 导表校验。资源不再执行字符串名称格式校验。

| Bean 字段 | 类型与约束 | 含义 |
| ---- | ---------- | ---- |
| `resource` | Luban `int` / Rust `i32`，允许完整有符号范围 | 游戏资源 ID，如 5001；不是 GameplayTag 或名称池内部索引 |
| `amount` | `long`，范围 `1..=4294967295` | 固定整数数量，编译时安全转换为运行时 `u32` |

技能 1002（InventoryBomb）内联一项 resource 为 `5001`、amount 为 1 的成本，
同时通过 `cost_effect_id=2001` 消耗 20 Mana，无冷却，启动时仅执行 EndAbility。
这个样例验证支付流程，不生成弹体。

同技能有多种成本时追加列表项，按填写顺序传给 Provider。相同资源和完全相同的项都允许
重复，编译器保留每项，不自动合并、排序或去重；Provider 必须对整批成本累计检查。
编译器提前拒绝每个技能同资源合计超过 `u32::MAX` 的配置，累计不跨技能进行。
不同技能填写相同值仍是独立配置副本，修改一处不会修改其他技能。当前配置数量固定，
不随等级变化；游戏 Provider 仍可通过现有 context 使用自己的规则。

必需检查在最小构建中也执行：每项数量处于正整数范围，每技能同资源 ID 的数量用 checked
加法累计并拒绝溢出。默认和最小构建都不额外限制资源 ID 的正负。编译错误定位包含 Ability
表、技能 ID 和从 0 起的列表位置，例如数量错误的 `additional_costs[1].amount`；累计溢出
定位到触发溢出的项。资源 ID 不再经过注册，因此不会产生资源名称注册错误。

GAS `AdditionalCost` 的 resource 本身就是 i32。模板按原列表顺序将表内 ID 直接传给
`AdditionalCost::new(resource, amount)`，`resource()` 和资源相关错误字段也直接使用该整数。
编译器中的 `resources` 映射已删除，无需把 ID 转成字符串或 `UniqueName`。
准备阶段保留的 `totals: BTreeMap<i32, u32>` 只用于启动时校验单个技能同 ID 数量累计是否溢出，
不参与资源注册，也不用于重排、合并或构造输出列表。标签和属性仍通过 `UniqueNamePool`
及各自管理器注册，原有快照构建与失败原子性保持不变。

游戏 Provider 直接匹配 5001 等整数 ID，或用它查询自己的资源字典，不需要名称池和资源键缓存。
表格不创建背包，也不自动安装 Provider；整数 ID 的类型合法不代表游戏存在对应道具，
未知资源仍由 Provider 拒绝。
使用基础启动器时，将 main 中的 `SimulationPlugin::new(settings)` 替换为：

```rust
SimulationPlugin::new(settings).with_additional_costs::<InventoryCosts>()
```

同时在游戏插件初始化 Provider 所需的背包组件或资源，不再重复安装 GAS。自行组合底层
插件的应用仍可使用 `GameplayAbilitySystemPlugin::with_additional_costs::<InventoryCosts>()`。

`InventoryCosts` 是游戏实现的 `AdditionalCostProvider`，实际检查、扣减和补偿
`context.source` 的库存。默认 Provider 遇到非空成本会拒绝激活。实现参考
[本游戏额外成本集成测试](../tests/config_test/additional_cost_test.rs) 中的 `InventoryProvider`，
以及 [GAS 依赖库的额外消耗协议](../../bevy_gas/.docs/14-extending-the-system.md#接入背包等额外消耗)。

修改表格后，在仓库根目录执行：

```powershell
pwsh -NoProfile -File config/export.ps1
cargo run --features config-validation --bin gas-config -- inspect assets/config 1002
cargo test --test config_test configured_duplicate_costs_reach_the_inventory_provider_without_deduplication
```

配置 CLI 可预览技能 1002。`examples/configured_game/` 演示技能 1001，尚未安装背包 Provider，
因此不会直接运行 InventoryBomb。上面的集成测试使用游戏自己的生成类型与编译器，
验证配置的整批额外消耗确实交给 Provider 检查、扣减，并在余额不足时拒绝支付；
失败后的补偿行为由 [GAS 额外消耗测试](../../bevy_gas/tests/gas_test/abilities_test/additional_cost_test.rs)
覆盖。接入背包玩法时在游戏中实现并安装 Provider。

额外消耗内联后只导出七张表，成本跟随 `gas_tbability.bytes`，不再生成或要求
`gas_tbabilityadditionalcost.bytes`。表集合与 Ability 字段布局均已改变，旧二进制包不能
与新生成的 Rust 模块混用，关闭 manifest 校验也不提供兼容转换。必须完整重新导表并配套
部署生成代码、七个 `.bytes` 和 manifest，不能只重写 manifest。无需新增物品表或 GAS 标签。

## 配置迁移

### 从资源名称迁移为整数 ID

1. 先在游戏内确定原资源名称到 i32 ID 的映射，例如 `Inventory.Bomb` → 5001、
   `Inventory.Ammo` → 5002；这些数字只是本模板示例，不是 GAS 预定义枚举。
2. 将每项 resource 替换为映射后的整数，保持列表顺序、重复项和 amount。若多个旧名称
   被映射到同一个 ID，应重新核对合计数量，因为它们现在属于同一种资源。
3. 同步 Provider 的资源字典、匹配逻辑和错误字段，直接使用同一组 i32 ID；移除额外成本
   的名称池注册、转换及键缓存。库存按整数 ID 查询，列表顺序与重复项仍原样交给 Provider。
4. 运行 `config/export.ps1` 完整重新导表，配套更新生成 Rust、全部 bytes 和 manifest。
   旧字符串 resource 的二进制布局不兼容新的 i32 字段，不能仅替换 manifest；关闭清单校验
   也不提供旧包兼容。数量、整批检查、扣减和补偿协议保持不变。

若配置已采用 resource:int，本次仅将 GAS 运行时资源字段统一为 i32，不改变 Excel、Schema
或生成二进制布局；无需为此再次改表或重新导表。更新 GAS 依赖与 Provider 代码后重新编译即可。

### 从独立成本表迁移为技能内联列表

1. 对每个技能，按旧 `additional_cost_ids` 的列表顺序查找成本行，将每行的 resource、amount
   展开到该技能的 `additional_costs`，原字符串 resource 按上面的游戏映射转成整数。
   不复制成本行 id，不按资源 ID 重排列表，也不合并重复项。
2. 原先共享同一成本行的多个技能各自得到一份内联值；迁移后它们独立修改。迁移时应确认
   展开后的顺序、数量和每技能同资源合计与旧配置一致。
3. 删除技能表的 `additional_cost_ids` 列，移除独立 `TbAbilityAdditionalCost` 表登记及
   `config/tables/gas.ability_additional_cost.xlsx`。在 XML 中保留同名 `AbilityAdditionalCost`
   作为只有 resource:int、amount:long 的 Bean，供技能内联列表使用。
4. `task_ids`、`modifier_ids` 及动作/修改器子表保持原有引用和顺序规则，不随成本内联调整。
5. 运行 `pwsh -NoProfile -File config/export.ps1`，由生成器重新导出 Rust 模块和七表配置包。
   旧的独立成本 bytes 不再部署；新生成代码、全部 `.bytes` 和 `manifest.json` 必须来自同次导表。
   旧包不能只换清单，迁移后继续验证预览、最小构建和 Provider 实际支付。

### 更早的子表反向关联布局

如果来源版本仍在子表记录父 ID 和 order，可直接迁移到当前布局：

1. 旧 Task 按 ability_id 分组、按 `(at_tick, order)` 排序，将 ID 写入技能 task_ids；删除
   Task 的 ability_id、order，保留 at_tick 与动作载荷。
2. 旧 AdditionalCost 按 ability_id 分组、按 order 排序，将 resource、amount 直接写入
   技能 additional_costs，资源名称按游戏映射转成整数；无需先生成成本 ID 列表，随后移除独立成本表。
3. 旧 Modifier 按 effect_id 分组、按 order 排序，将 ID 写入效果 modifier_ids；删除
   Modifier 的 effect_id、order。Task 动作中的 effect_id 仍表示待应用效果，保持不变。
4. task_ids/modifier_ids 仍使用分号分隔，清理重复 ID 和缺失引用；内联成本保留重复项。
   原有合法顺序、EndAbility 位置和每技能同资源合计保持不变，再完成上述全量导表与验证。

调整 Excel 物理行顺序不改变列表中的玩法顺序；修改列表顺序则改变同 tick 动作、
成本或修改器的定义顺序。动作/修改器仍可共享配置行，额外成本由各技能独立拥有。

## 加载、注册与授予

直接从 `bevy_gas_template::config` 导入运行时配置 API，无需启用 feature。
`load_tables(directory)` 始终通过 `read_package` 读取表数据，再交给 `generated::Tables::new`。
最小构建使用标准库按 `TABLE_FILES` 读取受大小限制的 `.bytes`，不解析清单、计算摘要或检查 schema。
启用 `config-validation` 时，先检查 manifest、schema、大小和摘要，再将同一次读取并验证的 bytes
交给解码器，不在校验后重新读取文件。

启用 `config-validation` 后，`validate_tables(&tables)` 检查名称、引用、未使用参数、
成本/冷却、动作顺序与公式；`compile_catalog(&tables, &mut world)` 在注册前使用相同校验，
直接复用已经完成的 `PreparedTables`，不重复准备。最小构建跳过完整制作规则，
两种构建均保留动作载荷、实际使用的幅度参数、所需引用和运行时注册状态检查。
未被技能或效果引用的动作、修改器和额外成本行允许保留；最小构建不使用它们，完整制作
校验仍检查其自身字段和引用，涉及父定义的策略按实际引用的父列表执行。
编译配置的 World 应先安装 GameplayAbilitySystemPlugin；仅编译定义不要求空间索引。
实际执行目标请求的应用另装游戏 TargetingPlugin。

```rust
use bevy_gas_template::config::{compile_catalog, load_tables};

let tables = load_tables(data_directory)?;
let catalog = compile_catalog(&tables, app.world_mut())?;
app.world_mut().insert_resource(catalog);
```

`compile_catalog` 返回 `Err` 时，World 中的名称池、标签表、属性表及已有 catalog 均保持原状。
实现先克隆三个注册表，在私有快照上完成名称登记与全部 Arc 定义构建，成功后才统一提交
三个 Resource；准备期间不会从 World 移除资源。快照同时承担注册预检和最终提交内容，
因此不需要维护两套容量/冲突判断，也没有失败回滚分支。

同名现存标签必须具有与配置一致的完整继承位图，同名属性必须属于一致的 Hot/Cold 区域。
容量不足、继承冲突、区域冲突或后期构建错误均丢弃快照，调用方可修正配置或加载备用配置，
失败尝试不会消耗后续名称与内部 ID。成功注册仍保留已有条目并追加新名称。

快照会复制启动时的注册表数据；这是为失败原子性付出的冷路径成本，不进入战斗热路径。
调用方只在 compile_catalog 成功后插入 catalog Resource。首版仍只支持启动加载，不替换战斗中的 catalog。

- 按名字排序登记 Tag/Attribute，祖先先于子标签，避免依赖 HashMap 或 Excel 行顺序。
- 固定注册顺序只保证相同配置的重复结果，不保证不同配置版本的内部编号相同。
- 按 EffectId 创建唯一 Arc，成本、冷却和任务动作复用它。
- 不按内容去重两个不同 EffectId；内容相同不代表叠层身份相同。
- 存档保存稳定配置 ID/名字，不保存 GameplayTag 位编号、AttributeId 或 AbilitySpecHandle。
- `grant_ability` 将共享定义授予 ASC，并写入同一角色的 ConfiguredAbilities。
  底层 `give_ability` 返回 `Result`；句柄耗尽会映射为
  `ConfigErrorKind::AbilityHandleExhausted`，保留 Ability 行定位，且不修改已有技能或配置绑定。
- `revoke_ability` 成对清除规格和映射；活动中的技能拒绝撤销并保留映射，已由底层清除的
  陈旧映射可以清理。需要再次授予时，先使用该接口完成撤销。
- 输入系统通过 ConfiguredAbilities 查 Handle，通过 CompiledAbility 的 targeting 向游戏 TargetingRequestQueue 发起请求，
  游戏插件的 continuation 再进入 GAS GameplayExecutionQueue。场景实体显式挂载 LogicalPosition，
  请求原点也取自逻辑位置；渲染 Transform 不作为索敌依据。

成本与冷却在激活时提交；技能结束不会自动移除已应用效果。取消后自动退款和延迟 Commit
不属于本配置层的行为。

## 结构化错误

所有配置层失败使用 `ConfigError`：`kind()` 返回可匹配的 `ConfigErrorKind`，
`location()` 返回 `ConfigLocation`，`message()` 和 `Display` 提供可读说明。
旧的字符串 `context()` 已移除，调用方不应解析文案来决定恢复流程。

| 类别 | 含义 |
| --- | --- |
| `Io`、`Capacity`、`Decode`、`Package` | 文件读写、边界限制、二进制解码、清单/schema/摘要或生成表契约 |
| `Validation`、`Reference`、`InvalidValue` | 制作规则、构建所需引用、运行时不可表示的值 |
| `MissingResource`、`Registration` | ECS 注册表缺失、名称/继承/区域注册失败 |
| `UnknownAbility`、`UnsupportedLevel`、`AlreadyGranted`、`AbilityHandleExhausted`、`ActiveAbility` | 查询/授予/撤销时可分别处理的调用状态，包括角色技能句柄耗尽 |
| `Report` | 文本报告格式化失败 |

位置用枚举明确区分文件、表、Resource 和操作。表位置包含表名、可选的行 ID/稳定名、
字段路径；文件位置包含完整读取路径、可选字段路径，以及解码失败的字节偏移。
例如加载失败可匹配 `ConfigLocation::File` 并定位实际出错的 `gas_tb*.bytes`，
不再只得到目录；校验等级可匹配 `ConfigLocation::Table` 的 Ability 行和 level 字段。
完整校验仍可在编译前拒绝制作规则；运行时所需检查不会依赖 feature 是否启用。

## 包版本与生成模板

导表生成的 manifest.json 包含格式/模板版本、结构摘要、整包内容摘要与每个文件的大小及摘要。
结构描述包含生成字段顺序、字段类型、普通枚举判别值、表主键及输出名称；
启用 `config-validation` 的加载器据此拒绝结构不兼容的数据。最小构建的加载器不执行这一兼容性检查，
部署时应配套使用同次导表生成的 Rust 模块和表数据。
数据内容摘要标识具体配置版本，普通数值变动允许由兼容的同一份读取代码加载；包协议和模板不因 feature 改变。

BLAKE3 与 serde_json 是由 `config-validation` 启用的可选直接依赖，分别负责摘要与清单 JSON。
关闭 feature 时，本游戏配置层不编译相关哈希、清单解析和校验代码；二进制读取与解码使用标准库。
Bevy 的默认 feature 可能独立引入同名传递依赖，因此整个依赖树中仍可能出现这些 crate。

两种构建均限制单文件 64 MiB、表数据合计 256 MiB；二进制集合和字符串另有上限。
启用 `config-validation` 时，另限制清单 1 MiB，要求恰好包含全部预期表，
拒绝重复/未知路径、结构不兼容和摘要不一致。内容摘要用于一致性检查，不承担发布者签名认证。

模板支持基础类型、Option、Vec、普通枚举和非继承 map 表。
所有 Decode 返回 Result，非法枚举、截断、长度错误、重复主键、尾随字节均返回错误。
不调用上游旧的不可失败 deserialize helper，不使用 flags、抽象 Bean 或 dyn Any。
变更模板后重新导表，禁止手改生成 Rust 来修复当前结果。

## 验证与维护

本页配置源码属于游戏 `bevy_gas_template` 包，GAS 核心通过 Cargo 依赖提供。常规检查在仓库根目录执行：

```powershell
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
cargo build
```

`cargo test` 默认即验证包校验和完整业务校验；`cargo test --all-features` 与默认构建一致，仍保留为回归口径。
配置测试通过 `tests/config_test.rs` 加载 `tests/config_test/`；位置、索敌、命令与共享模拟集成测试
由 `tests/gameplay_test.rs` 加载 `tests/gameplay_test/`。
这些基础测试自建表数据和临时配置包，不依赖导表生成的 `assets/config/` 或 examples。配置演示专用测试留在 `examples/configured_game/`，导表后使用 `cargo test --all-features --examples` 和 `cargo test --no-default-features --examples` 显式执行。
配置变更使用完整导表验证候选 Rust 模块、真实二进制包和 GAS 语义，再运行上面的
配置预览、无窗口验收与额外成本集成测试检查实际行为。修改 Excel、schema 或模板后，必须重新导表并同步提交
源文件和生成 Rust 模块；不手动修改 manifest 来掩盖结构或数据变化。
根目录 `target/` 是可删除的构建缓存，后续 Cargo 命令会重新创建。

额外消耗配置回归测试位于 `tests/config_test/additional_cost_test.rs`，覆盖生成数据解码、
内联列表顺序、重复项和整数资源身份、每技能独立累计数量、下标错误定位、非法配置、失败时状态保持，以及配置技能
通过游戏 Provider 实际支付。

有关工具安装和 MCP 的细节见 [02 — Luban 工具链](./02-luban-toolchain.md)。

配置编译回归测试位于 `tests/config_test/compilation_test.rs`：覆盖已初始化注册表冲突后的
资源保持与重试、后期构建失败不影响已有 catalog、缺失 Resource 分类，以及输入行序变化时
编译任务与预览时间线顺序一致，并验证显式 EndAbility 约束。
父列表引用回归测试位于 `tests/config_test/references_test.rs`：覆盖 task_ids/modifier_ids 的重复与缺失引用、
共享修改器的独立顺序、新二进制字段布局，以及未引用定义的完整业务校验。
最小构建和默认构建均需执行这些外部行为测试。

`runtime_test.rs` 另通过真实配置编译和 GAS 效果应用，验证 LinearLevel 正/负溢出在 Instant 与
Duration 效果中返回明确错误、保持原属性且不创建活动效果；完整制作校验仍提前拒绝允许等级范围内的溢出。
