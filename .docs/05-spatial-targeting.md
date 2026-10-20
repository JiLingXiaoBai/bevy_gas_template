# 05 — 空间、索敌与 GAS 接入

## 所有权与当前范围

模板按可组合能力组织游戏空间逻辑，不用 RTS/MOBA/ACT 枚举分支驱动一套固定流程：

| 模块 | 所有权 | 不负责 |
| --- | --- | --- |
| `gameplay::position` | 逻辑世界位置、可选的位置表现同步与插值 | 索敌规则、技能和效果结算 |
| `gameplay::spatial` | crate 内部动态 AABB 树与派生边界 | GAS 状态、标签、锁定或伤害规则 |
| `gameplay::targeting` | 目标定义、空间索引同步、选择/过滤/排序、目标请求队列与 `TargetingPlugin` | 武器碰撞、效果应用和技能合法性判断 |
| `gameplay::GameplaySet` / `GameplaySchedulePlugin` | 独立声明并排序游戏移动、空间同步、索敌三个固定 tick 阶段 | GAS 内部效果与技能执行、索敌资源安装 |
| `bevy_gas::gas::gameplay_targeting` | `AbilityTargetHit`、`AbilityTargetData`、`AbilityActivationTargets` 结果契约 | 如何选择目标或确认命中 |

本次拆分迁移既有实现和行为，没有新增第三方依赖，也没有新增碰撞检测。
当前范围选择仍按逻辑位置点判定；Fat AABB 是更新余量，不是单位受击体。
动作游戏所需的攻击体/受击体、体积相交、扫掠、攻击窗口、部位归属和重复命中规则应由游戏层
进一步实现，再通过公开结果与效果队列接入 GAS。索敌成功不代表武器已经接触目标。

目标数据和激活值由 GAS 库定义，模板只生成和使用这些值，不复制一份类型。
结果的构造、顺序与多目标效果语义见
[GAS — 目标结果契约](../../bevy_gas/.docs/15-gameplay-targeting.md)。

## 安装与迁移

GAS 插件不再安装空间索敌或位置表现。模板应用显式组合所需插件：

```rust
use bevy::prelude::*;
use bevy_gas::prelude::*;
use bevy_gas_template::gameplay::{
    position::PositionPresentationPlugin,
    targeting::TargetingPlugin,
};

app.add_plugins(GameplayAbilitySystemPlugin);
app.add_plugins((TargetingPlugin, PositionPresentationPlugin));
```

`TargetingPlugin` 接入 GAS 的公开阶段和 `GameplayExecutionQueue`，应与 GAS 运行时一起安装。
它会在尚未安装时添加 `GameplaySchedulePlugin`，既有应用无需修改安装方式。
只有移动、没有索敌的应用可单独安装 `GameplaySchedulePlugin`，仍获得
`EffectTicks → Movement → AbilityTasks → RequestProducers` 的顺序，不创建索引或索敌队列。
若同时显式列出两个插件，应先安装 `GameplaySchedulePlugin`，再安装 `TargetingPlugin`，遵循
Bevy 的唯一插件规则。只声明 SystemSet 或给系统标记集合并不会自动建立这些顺序。
`PositionPresentationPlugin` 可以单独使用，不依赖 GAS 或索敌；纯逻辑服务器可以不安装它。
需要视觉子节点的应用另安装 Bevy `DefaultPlugins` 或 `TransformPlugin`。

迁移旧调用方时，将位置 API 导入改为 `bevy_gas_template::gameplay::position`，索敌实现 API
改为 `bevy_gas_template::gameplay::targeting`，目标结果继续从 `bevy_gas` 导入。
`GameplayAbilitySystemSet::Movement` / `TargetingSpatialSync` / `Targeting` 分别替换为
`GameplaySet::Movement` / `SpatialSync` / `Targeting`，并显式安装上述插件。
游戏配置编译器构造本包的 `TargetingDefinition`；数据表与生成格式不因模块迁移改变。

## 源码所有权

以下路径相对于 `src/gameplay/`；新模块均使用门面文件与同名目录：

| 路径 | 职责 |
| --- | --- |
| `schedule.rs`、`schedule/phases.rs`、`schedule/plugin.rs` | 游戏阶段与独立 `GameplaySchedulePlugin`，只声明相对 GAS 的顺序 |
| `position.rs`、`position/` | 逻辑位置、可选表现组件、错误与独立表现插件 |
| `spatial.rs`、`spatial/dynamic_aabb_tree.rs` | crate 内部动态 AABB 树；不依赖索敌标签或 GAS 执行 |
| `targeting/targeting_definition.rs` | Targetable 与目标操作定义、验证 |
| `targeting/acquisition.rs` | 候选精筛、过滤、排序、限制与同步抓取 SystemParam |
| `targeting/spatial_index.rs`、`targeting/spatial_index/index.rs` | 派生空间索引、变化同步和组件生命周期 |
| `targeting/targeting_queue.rs`、`targeting/targeting_queue/` | 输入、结果 Event/Message、FIFO、continuation 及批次处理 |
| `targeting/plugin.rs` | TargetingPlugin 资源与系统安装、空间生命周期 Observer |
| `targeting/error.rs` | 目标定义与运行时抓取错误 |
| `../application.rs`、`../application/` | 共享模拟设置与 GAS/游戏调度装配，不创建场景或读取配置 |
| `../game.rs`、`../game/` | 由基础启动器注册的游戏业务入口 |
| `examples/configured_game/`（仓库根目录） | 独立配置演示、窗口与无窗口验收及其测试 |

空间索引只保存派生数据。逻辑位置、标签、属性和技能状态仍由各自 ECS Component 持有。
`TargetingRequest` 是私有实现；调用方使用输入、定义、continuation、请求 ID、结果 Event/Message 和队列。
`TargetingResultEvent` 保持原有延迟 Observer 行为；新增的 `TargetingRequestResult` 在处理系统内
作为 Message 发布，并携带 continuation 的提交结果与 GAS 执行请求 ID。
需要从玩家/AI 意图追踪至最终激活结果时，使用 [08 — 游戏命令](08-game-commands.md)。

## 定义与实体要求

每个 `TargetingDefinition` 必须以恰好一个 Selection 开头，后续操作按给定顺序细化结果：

| 类别 | 操作 | 行为 |
| ---- | ---- | ---- |
| Selection | `SelectSelf` | 选择来源；不要求 `Targetable` |
| Selection | `SelectExplicitEntity` | 选择输入中的显式实体；要求 `Targetable` |
| Selection | `SelectSphere` | 选择球形范围内的 `Targetable` 实体 |
| Selection | `SelectCone` | 选择方向锥形范围内的 `Targetable` 实体 |
| Filter | `FilterSource` | 排除来源 |
| Filter | `FilterTags` | 使用 `TagRequirements` 过滤 |
| Filter | `RequireAttributeSet` | 要求目标具有 `AttributeSet` |
| Filter | `FilterDistance` | 要求距请求原点不超过上限 |
| Sorting | `SortByDistance` | 按距离平方升序或降序排序 |
| Limit | `Limit` | 截断为前 N 个 |

`TargetingDefinition::new()` 拒绝空操作、Selection 不在首位、多个 Selection、负数或非有限
radius/distance、非有限或超出 `[0, PI]` 的半角，以及 `Limit { count: 0 }`。

操作顺序具有语义。通常应先过滤，再排序，最后 Limit；先 Limit 再排序只会重排已截断集合。

候选 Query 要求实体有 `LogicalPosition`。Sphere、Cone 和 Explicit selection 还要求
`Targetable`；`SelectSelf` 只要求来源有 `LogicalPosition`。`FilterTags` 配置非空时，没有
`GameplayTagContainer` 的候选不会通过；空 requirements 对有无容器都通过。

## 逻辑位置与可选表现

`gameplay::position` 模块的 `LogicalPosition(pub Vec3)` 保存权威世界坐标。移动、索敌范围、
距离排序和 Hit 位置只读取这一组件；索敌不读取 `Transform` / `GlobalTransform`，也不会在
缺少逻辑位置时回退到表现坐标。`TargetingInput::origin` 仍由调用方显式传入，施法者原点应从
逻辑位置取得，避免主动把插值后的表现位置传回 Gameplay。

`LogicalPosition` 不 Required `Transform`，也不依赖 TransformPlugin。服务器或无表现对象
只需显式组合逻辑位置和 `Targetable`；完整 GAS Actor 另外组合 `GameplayAbilitySystemBundle`。
逻辑位置始终是世界坐标，不继承 `ChildOf` 父实体的变换，移动父节点不会隐式改变子节点的逻辑位置。

需要显示的实体可增加 `PositionPresentation`，它 Required `Transform`，在每帧 PostUpdate
只更新 `Transform.translation`，保留 rotation 和 scale。默认和 `interpolated()` 使用插值；
`direct()` 直接采用当前逻辑位置。`is_interpolation_enabled()` 查询模式；
`set_interpolation_enabled(bool)` 在模式变化时清除历史，下一次表现使用当前位置。

```rust
use bevy::prelude::*;
use bevy_gas_template::gameplay::{
    position::{LogicalPosition, PositionPresentation},
    targeting::Targetable,
};

// A simulation-only target needs no transform components.
commands.spawn((LogicalPosition(Vec3::new(5.0, 0.0, 0.0)), Targetable));

// A visual root interpolates its translation from authoritative logical positions.
commands.spawn((
    LogicalPosition(Vec3::new(5.0, 0.0, 0.0)),
    Targetable,
    PositionPresentation::interpolated(),
));
```

位置 API 从 `bevy_gas_template::gameplay::position` 显式导入。
表现系统由独立 `PositionPresentationPlugin` 安装；游戏需要时显式添加，GAS 插件组不包含它。
表现根必须没有 `ChildOf`；Mesh、Sprite 等可以作为其视觉子节点，
由 Bevy 官方变换传播系统处理。表现应用需安装 `DefaultPlugins` 或 `TransformPlugin`，以便
更新 `GlobalTransform`；纯逻辑应用无此要求。当前不支持把逻辑世界坐标直接写入有父级实体的
局部 `Transform`，也没有内建独立逻辑实体到另一个表现实体的映射。

### 插值时序与传送

表现插件在每次 `FixedFirst` 运行 `capture_logical_position_history_system`，记录本
fixed tick 开始时的位置；Gameplay 移动在 `FixedUpdate` 的 `GameplaySet::Movement` 阶段修改逻辑位置。每帧 `PostUpdate`
运行 `sync_position_presentation_system`，并排在 `TransformSystems::Propagate` 之前：

```text
FixedFirst：保存 previous
→ FixedUpdate：游戏 Movement → GAS AbilityTasks → GAS RequestProducers → 游戏 SpatialSync/Targeting → GAS 结算
→ PostUpdate：lerp(previous, current, Time<Fixed>.overstep_fraction())
→ Bevy Transform 传播：更新表现根与视觉子节点
```

常规插值的表现比最新模拟状态延迟一个 fixed tick，这是在两个已知状态间平滑过渡的预期行为。
一帧零个 fixed tick 时保留历史，只推进插值比例；一帧多个 tick 时使用最后相邻的两个逻辑状态。
首次生成或没有历史时直接同步当前逻辑位置；没有 `Time<Fixed>` Resource 时也直接同步。

传送时同时修改逻辑位置并清除表现历史，下一次表现同步直接落在新位置，避免穿过中间路径：

```rust
position.0 = destination;
presentation.reset();
```

`position` 与 `presentation` 分别是目标的可变 `LogicalPosition` 和 `PositionPresentation`。
若希望传送影响当前 tick 的索敌，应在 `GameplaySet::Movement` 阶段完成逻辑位置修改，再由后续请求生产
系统采样 origin；不要在空间同步与范围抓取之间改位置。

常规连续移动通过 `Mut<LogicalPosition>` 修改 `.0`。表现插件注册的私有 Discard Observer
在移除或 `insert` 替换 `LogicalPosition` 时自动清除表现历史，组件替换视为不连续移动，
同一帧移除再添加也不会沿用旧位置插值。通过 Mut 修改坐标不会触发 Discard，因此这种方式的
传送仍需显式调用 `presentation.reset()`。

`PositionPresentation::last_error()` 返回最近历史采集或表现同步检测到的可选
`PositionPresentationError`，成功采集或同步后清除错误。`reset()` 只清历史，不修改位置、
Transform 或已有错误。
缺失逻辑位置、缺失 Transform、逻辑位置非有限或实体带有父级时，分别记录
`MissingLogicalPosition`、`MissingTransform`、`NonFiniteLogicalPosition`、`ParentedEntity`，
并清除历史、保持原 Transform。修复组件或坐标后可在后续同步恢复；表现错误不会修改权威逻辑位置。

读取本帧表现根 `Transform` 的跟随相机等系统应安排在 `PostUpdate`，显式
`.after(sync_position_presentation_system).before(TransformSystems::Propagate)`。
若读取的是传播后的子节点 `GlobalTransform`，则需排在传播之后，不能把尚未传播的世界变换
当作本帧的表现结果。

## 动态 AABB 树

Sphere/Cone 的候选来源统一为 `TargetingSpatialIndex` 动态 AABB 树，不保留全量扫描后端或
扫描回退。每次范围查询先用 `origin ± radius` 的 AABB 粗筛，再从 ECS 读取候选的当前位置，
完成原有球形距离、锥形角度和后续操作。Self/Explicit 已知实体，继续直接通过 Query 查询。

索引只保存 Entity 与派生空间数据；标签、属性和 Gameplay 状态仍由 ECS Component 持有。
目标仍按 `LogicalPosition.0` 表示的位置点建模，Fat AABB 只是适当扩大的
树代理，当前每轴外扩 0.5 世界单位，不是单位碰撞体，也不扩大技能命中范围。目标在代理盒内移动时无需重新插入叶子，
超出后才更新树结构；精筛始终读取当前 ECS 位置。

`sync_targeting_spatial_index_system` 首次建立索引，随后通过组件变化检测增量同步
`Targetable` / `LogicalPosition`。`TargetingPlugin` 还注册两种组件的 Discard Observer，在组件
移除、替换或实体销毁时清理旧代理，不依赖 RemovedComponents 消息跨 fixed tick 保留。
同步系统也处理可读到的 RemovedComponents，支持手动调度；常规接入应安装 `TargetingPlugin`
获得完整生命周期处理。无目标请求时仍运行同步。非有限位置不进入树，恢复为有限位置后会重新
加入。树遍历顺序不参与 Gameplay 语义，候选仍先按 Entity bits 排序，再执行管线。

首次同步会遍历所有合格实体建立索引；后续同步只修改变化实体的树代理，但 ECS 的 Changed
过滤仍需检查组件 change tick。消除的是每次范围请求的全场景扫描，不是索引维护的全部遍历。

`TargetingSpatialIndex::len()` / `is_empty()` 反映当前代理数量；`is_ready()` 只表示首次同步已
完成，不验证游戏层移动与索敌的调度顺序，也不保证同步后没有发生空间修改。

## 同步抓取 API

```rust
pub fn acquire_targets(
    source: Entity,
    input: TargetingInput,
    definition: &TargetingDefinition,
    params: &TargetingSystemParams,
) -> Result<AbilityTargetData, TargetingError>;
```

`TargetingInput::new(origin, direction)` 捕获请求提交时的空间输入，
`with_explicit_target(entity)` 附加显式目标。Origin 总会检查有限值；Direction 仅在 Cone selection
需要时检查有限且平方长度大于 `f32::EPSILON`。缺失实体/Component、无效输入和最终空集合都会返回具体
`TargetingError`，不会 panic。

`TargetingSystemParams` 同时读取 `TargetingSpatialIndex` 和候选 ECS Query；同步调用与队列
处理使用同一入口。调用方必须先执行空间同步，再抓取，且两者之间不再修改目标空间状态。
未完成首次同步时，Sphere/Cone 返回 `TargetingError::SpatialIndexNotReady`，不会扫描 ECS
作为补偿。Self/Explicit 不依赖索引就绪状态，但位置非有限时返回 `InvalidTargetPosition`。

同步抓取系统注入 `TargetingSystemParams`，并将 `&params` 传给 `acquire_targets()`。
低层 `TargetingCandidateQuery` 别名仍可用，但不单独构成同步抓取的访问参数。

## 队列 API 与 continuation

```rust
pub fn push_request(
    &mut self,
    source: Entity,
    input: TargetingInput,
    definition: Arc<TargetingDefinition>,
    continuation: TargetingContinuation,
) -> TargetingRequestId;
```

`TargetingRequestQueue` 还公开 `len()`、`is_empty()` 与 `clear()`；出队只属于内部 processor。
请求 ID 从 1 开始，按入队顺序分配，溢出后跳过 0。

```rust
pub enum TargetingContinuation {
    EmitResult,
    ActivateAbility {
        handle: AbilitySpecHandle,
        context: Box<AbilityActivationContext>,
    },
}
```

推荐使用 `TargetingContinuation::activate_ability(handle, context)` 创建激活 continuation。
continuation 刻意只保存 handle 与 Context：请求创建时尚未完成 target acquisition，因此不能
提前构造要求 targets 已存在的 `AbilityActivationData`。

processor 完整 drain 当前 Targeting FIFO。每个请求严格执行：

1. 调用同步 `acquire_targets()`。
2. 成功且 continuation 为 `ActivateAbility` 时，通过 `AbilityActivationTargets::acquired()`
   把完整 Target Data 转换为唯一目标值，再把 Targeting Request 的 source、该 targets 与
   continuation Context 组装为 `AbilityActivationData`，据此写入 `GameplayExecutionQueue`。
3. 通过 `Commands::trigger()` 排队触发包含同一 `TargetingRequestId` 的
   `TargetingResultEvent`，无论成功或失败都会触发。
4. 处理下一个 Targeting 请求。

Direct continuation 在结果 Observer 之前已经入队，Observer 不能撤销它。需要审核、玩家确认
或取消时使用 `EmitResult`，再由 `On<TargetingResultEvent>` Observer 决定是否生产 Gameplay
请求。

整批 direct continuations 都在 processor 内完成；deferred result triggers 在系统返回后才应用，
所以 Observer 派生的 Gameplay 请求排在该批所有 direct continuation 之后，不会逐请求交错。

## FixedUpdate 时序与边界

图中带 `GameplaySet::` 前缀的是游戏阶段，其余来自 GAS `GameplayAbilitySystemSet`。

```text
EffectTicks
    ↓
GameplaySet::Movement
    ↓
AbilityTasks
    ↓
RequestProducers
    ↓
GameplaySet::SpatialSync
    ↓
GameplaySet::Targeting
    ↓
PreGameplayConvergence
    ↓
GameplayResolve
    ↓
UpdateEffectTagRequirements
    ↓
Cleanup
    ↓
RecalculateAttributes
```

`GameplaySchedulePlugin` 将三个游戏阶段排序到 GAS 的公开阶段之间。`TargetingPlugin` 自动安装该插件，
初始化索引/FIFO、注册空间生命周期 Observer，并在 `GameplaySet::SpatialSync` 无条件运行同步系统。游戏层将本 tick 的 `LogicalPosition` 移动和
相关 deferred 命令放入 `GameplaySet::Movement`，在 `RequestProducers` 中读取已移动的位置提交请求。
阶段顺序保证这些命令在后续任务、请求采样及空间同步中可见；同集合内的业务顺序仍需游戏显式约束。
索敌不再依赖 FixedUpdate 内的世界变换传播。表现变换在 PostUpdate 更新，不会改变空间索引或抓取结果。
自定义同步抓取系统应显式排在 `GameplaySet::SpatialSync` 之后；需要进入本 tick 统一 Gameplay
FIFO 的结果还须排在 `GameplayResolve` 之前。

空间索引不是任意时刻实时更新的全局视图。同步后发生的移动、增删组件和实体生成，需等下一次
同步后参与范围抓取；不要在同步与范围抓取之间修改这些状态。标签和属性过滤仍读取抓取时的 ECS
状态，无需等待空间同步。

| 产生位置 | 结果 |
| -------- | ---- |
| `AbilityTasks` / `RequestProducers` 或其他明确位于 `GameplaySet::Targeting` 前的系统写入 Targeting FIFO | 当前 tick 抓取 |
| Targeting processor 的 direct activation continuation | 当前 tick `GameplayResolve` |
| `TargetingResultEvent` Observer 写入 Gameplay FIFO | 模板插件组合下仍在 `GameplayResolve` 前，当前 tick |
| `TargetingResultEvent` Observer 再写 Targeting FIFO | processor 已结束，下一 tick 抓取 |
| `GameplaySet::Targeting` 阶段之后新写 Targeting FIFO | 下一 tick 抓取 |

队列没有每 tick 数量上限，会完整处理进入本次 drain 的请求。Observer 在 processor 返回后新增
的 Targeting 请求不会被本次已经结束的 drain 重新消费。

## 多目标效果

- startup `Instant` 中的 `ApplyGameplayEffectToTargets` 在激活请求内部按动作定义顺序、再按
  `AbilityActivationTargets::entities()` 顺序直接应用。
- 运行时任务的同名动作按同一个目标值的顺序向统一 Gameplay FIFO 追加独立请求；single
  产生一个请求，acquired 按 Target Data 顺序产生多个请求。
- `ApplyGameplayEffectToTarget` 使用同一目标值的 `get_primary_target()`，主目标没有第二个存储源。
- 空 Target Data 不能创建 acquired 激活目标，因此运行时不存在 Context 与 Request 各自携带
  不同目标的分裂状态。
- 每个目标独立进行要求、免疫、概率和堆叠检查；一个目标拒绝不会中断后续目标的处理。

## 确定性与当前边界

- Sphere/Cone selection 先按 `Entity::to_bits()` 建立基础顺序。
- 距离排序使用 `f64::total_cmp()`，相同距离用 Entity bits 打破平局。
- 请求捕获 Origin、Direction 和显式目标，不在消费时重新读取输入设备。
- 非有限目标位置不进入范围候选；Self/Explicit 对无效位置返回具体错误。
- ECS 坐标与请求仍使用 `f32`；边界与距离计算使用 `f64` 中间值，避免有限 `f32` 坐标运算
  溢出。这不构成跨平台位级锁步保证。
- 当前没有物理射线/形状投射、遮挡检查、单位体积相交、纯世界位置目标、玩家确认任务或网络预测/复验。

动态树减少空间范围外的候选访问，但维护索引本身也有成本。性能评估应统计索引同步、范围查询、
精筛和排序的总成本，并改变实体数、移动比例、请求数与覆盖范围；不能仅凭使用树就承诺加速。

## 测试导航

| 测试文件 | 覆盖范围 |
| -------- | -------- |
| `tests/gameplay_test/gameplay_schedule_test.rs` | 无索敌时的移动顺序与 Commands 可见性、独立调度和索敌插件组合 |
| `tests/gameplay_test/gameplay_position_test.rs` | 独立表现插件、显式安装边界、逻辑/表现分离、插值、传送与组件替换、非法状态及视觉子节点传播 |
| `tests/gameplay_test/gameplay_targeting_test.rs` | 定义与几何边界、逻辑位置索引生命周期/有限值、同 tick 同步、稳定排序、完整 drain 与多目标激活 |
| GAS 的 `tests/gas_test/gameplay_targeting_test.rs` | 目标结果契约与多目标效果结算，不依赖模板空间实现 |

### 手动工作负载测量

以下 ignored 测试可显式运行；此命令使用默认 **debug 构建**：

```bash
cargo test --test gameplay_test measure_dynamic_aabb_targeting_workloads -- --ignored --nocapture
```

测试在规则网格上放置 64 / 4096 个目标，组合半径 12 / 512、移动比例 0% / 10% / 100%，
以及每 tick 1 / 32 次球形抓取并按距离排序。每组先预热一次，再取 4 tick 的平均耗时。
移动沿 Z 轴在 ±0.25 世界单位之间切换，留在当前 Fat AABB 内；因此这组测量主要覆盖
变化检测与代理盒内更新，不代表持续跨盒移动、频繁树重插入或生成销毁的成本。

输出单位为毫秒 / tick：`fixed_update_ms` 包括完整测试 App 的 FixedUpdate（含索引同步），
`query_sort_ms` 包括该 tick 的全部同步范围查询、精筛和排序，`total_ms` 为两者之和。
初始建树、目标位置写入以及请求队列/结果 Observer 派发不在这两段计时内。

这是动态树单实现的工作负载耗时，没有旧扫描实现作为对照，不能据此推导加速比。
少量 debug 采样也不等同于优化构建下的生产场景性能；比较时须固定机器、构建配置和场景，
另行覆盖实际游戏的移动模式与查询分布。

逻辑位置和插值示例在本仓库运行：

```bash
cargo run --example logical_position_targeting
```

位置测试的基础 fixture 只安装 MinimalPlugins、TransformPlugin 与 PositionPresentationPlugin，
覆盖零个/多个 fixed tick、首次无历史、传送 reset、组件替换、非法状态恢复和视觉子节点传播。
仅运行 FixedUpdate 不会采集 FixedFirst 历史；验证插值时显式推进 FixedFirst、FixedUpdate、PostUpdate。
索敌测试不依赖 TransformPlugin，使用已知目标构造明确的有序预期，不保留扫描后备或对照实现。
应覆盖组件新增/移除/替换、销毁与实体复用、有限值边界、移动与请求的同 tick 可见性。

继续阅读：[03 — 配置编译](./03-gas-configuration.md)、
[GAS — 统一执行](../../bevy_gas/.docs/16-gameplay-execution.md)。
