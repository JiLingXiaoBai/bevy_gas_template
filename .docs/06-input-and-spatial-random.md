# 06 — 输入绑定与空间随机

## 所有权与安装

输入绑定与空间随机由模板维护，GAS 仅接收已经解析的技能 Handle、目标和激活上下文。
本页两个模块可独立使用，不依赖索敌插件，也不自动安装任何插件或系统。

| 模块 | 公开 API | 职责 |
| --- | --- | --- |
| `gameplay::ability_input` | `AbilityInputBindings<Action>`、`AbilityInputBindingError` | 逻辑动作到同实体 ASC 的技能 Handle 映射 |
| `gameplay::randoms` | `SpatialRandom` | 使用独立随机流构造 Bevy 随机值并进行形状采样 |

类型从上述领域门面显式导入；GAS 的 prelude 和 crate root 不再提供输入绑定兼容导出。
需要跨帧缓冲和命令结果关联时，可组合独立的 `gameplay::commands`，见
[08 — 游戏命令与技能结果关联](08-game-commands.md)。它不改变本页绑定组件和独立随机流的职责。

## 输入绑定职责与数据流

`AbilityInputBindings<Action>` 是角色上的可选 Component，保存游戏自定义逻辑动作或技能槽到
`AbilitySpecHandle` 的映射。它与拥有这些技能的 `AbilitySystemComponent` 挂在同一个实体上。

```text
键盘 / 手柄 / UI
      │ 游戏输入层：设备映射与输入状态
      ▼
逻辑动作，例如 PrimaryAttack、Slot(0)
      │ AbilityInputBindings<Action>：解析当前绑定
      ▼
AbilitySpecHandle + 目标 + Input 激活上下文
      │ RequestProducers：加入 GameplayExecutionQueue
      ▼
GameplayResolve：验证并执行技能
```

| 层 | 保存的信息 |
| --- | --- |
| `GameplayAbility` | 可共享技能定义、消耗、冷却和任务 |
| `GameplayAbilitySpec` | 角色获授的 Handle、等级和活跃计数 |
| `AbilityInputBindings<Action>` | 该角色的逻辑动作或技能槽到 Handle 的映射 |
| 游戏输入层 | 物理设备映射、按下/释放/长按状态、输入缓冲和触发策略 |
| 游戏 UI 层 | 技能栏图标、名称、布局等展示配置 |

`GameplayAbilitySpec` 已移除 `input_id` 和 `input_pressed`；授予 API 为
`give_ability(ability, level)`。绑定组件不安装 `InputPlugin`、系统或 Required Component，
也不加入默认 `GameplayAbilitySystemBundle`。AI、脚本和其他生产系统仍可直接按 Handle 入队。

## 创建与绑定

`Action` 由游戏定义，只需满足 Component 的 `Send + Sync + 'static` 约束；按动作查询和修改
还要求 `Eq`，不要求 `Copy`、`Clone`、`Hash` 或 `Default`。技能栏可直接使用带槽位参数的枚举：

```rust
use bevy::prelude::*;
use bevy_gas::gas::ability_system::AbilityGrantError;
use bevy_gas::gas::gameplay_abilities::{AbilityTaskDef, AbilityTaskOnFinishedDef};
use bevy_gas::prelude::*;
use bevy_gas_template::gameplay::ability_input::AbilityInputBindings;
use std::sync::Arc;

#[derive(PartialEq, Eq)]
enum PlayerAction {
    PrimaryAttack,
    Slot(u8),
}

fn spawn_player(mut commands: Commands) -> Result<(), AbilityGrantError> {
    let mut ability_system = AbilitySystemComponent::default();
    let laser = Arc::new(GameplayAbility::default().with_startup_tasks(vec![
        AbilityTaskDef::instant(AbilityTaskOnFinishedDef::EndAbility),
    ]));
    let handle = ability_system.give_ability(laser, 1)?;

    let mut bindings = AbilityInputBindings::<PlayerAction>::default();
    let _ = bindings.bind(PlayerAction::Slot(0), handle);
    let _ = bindings.bind(PlayerAction::PrimaryAttack, handle);

    commands.spawn((
        GameplayAbilitySystemBundle {
            ability_system,
            ..Default::default()
        },
        bindings,
    ));
    Ok(())
}
```

一个动作最多绑定一个技能；多个动作可以指向同一份技能，因此上例两个动作共享等级、冷却和
活跃实例限制。需要独立技能规格时，应分别授予技能并绑定各自的 Handle。

组件内部使用连续的 `Vec`，面向通常较小的角色动作表做线性查找。读取与解析不分配内存，
遍历始终按插入顺序进行；新增绑定可能扩容。没有依赖 HashMap 遍历顺序的玩法语义。

## 绑定 API 与重绑语义

| API | 返回值与行为 |
| --- | --- |
| `bind(action, handle)` | 返回该动作原先的 `Option<AbilitySpecHandle>`；新动作追加，重绑保留原位置 |
| `get(&action)` | 返回配置中的 Handle；未绑定返回 `None`，不检查 ASC |
| `resolve(&action, &ability_system)` | 返回仍在指定 ASC 中的 Handle，或具体绑定错误 |
| `unbind(&action)` | 返回被移除的 Handle；其余动作保持相对顺序 |
| `unbind_ability(handle)` | 移除该 Handle 的全部绑定，返回移除数量 |
| `iter()` | 按插入顺序返回 `(&Action, AbilitySpecHandle)` |
| `len()` / `is_empty()` | 查询绑定动作数量；同技能的多个别名分别计数 |
| `clear()` | 清空绑定并保留分配容量 |

物理改键只调整设备到动作的映射；替换技能栏某一槽位的技能时，对该槽位再次调用 `bind()`。
移除动作后重新绑定会把它追加到末尾；如果 UI 需要按槽位编号显示，应使用槽位编号决定布局，
不要把绑定插入顺序当作槽位编号。

重绑或解绑不会取消已激活的技能，也不会改写已经进入 Gameplay FIFO 的请求。请求已经持有解析
完成的 Handle，因此按照提交时的技能执行；如果规格随后被移除，resolver 会拒绝该请求。

## 解析与请求生产

`resolve()` 区分两个错误：

- `AbilityInputBindingError::UnboundInput`：该动作没有绑定。
- `AbilityInputBindingError::AbilityNotGranted { handle }`：绑定存在，但 ASC 已没有对应规格。

解析仅检查规格存在性，冷却、消耗、标签和多实例限制仍由技能激活流程检查。下面展示从逻辑动作
直接组装请求的核心步骤；实际设备输入应按下一节跨帧缓冲，再由
`GameplayAbilitySystemSet::RequestProducers` 提交请求：

```rust
use bevy::prelude::{Entity, error};
use bevy_gas::prelude::*;
use bevy_gas_template::gameplay::ability_input::AbilityInputBindings;

fn enqueue_input<Action: Eq + Send + Sync + 'static>(
    action: &Action,
    source: Entity,
    target: Entity,
    bindings: &AbilityInputBindings<Action>,
    ability_system: &AbilitySystemComponent,
    queue: &mut GameplayExecutionQueue,
) {
    let handle = match bindings.resolve(action, ability_system) {
        Ok(handle) => handle,
        Err(error) => {
            error!("failed to resolve ability input: {error}");
            return;
        }
    };
    let chain = queue.new_root_chain(handle);
    let context = AbilityActivationContext::input(source, chain);
    if let Err(error) = queue.push_activation(
        source,
        AbilityActivationTargets::single(target),
        handle,
        context,
    ) {
        error!("failed to queue ability input: {error}");
    }
}
```

调用方应从同一个 `source` 实体查询 bindings 和 ASC，并根据游戏规则选取目标。
`AbilityActivationContext::input(source, chain)` 生成无编号的
`AbilityActivationReason::Input`；具体动作仍在输入层。需要属性快照时，与直接激活一样显式
调用 `with_source_snapshot()`。

同一阶段多个生产系统的请求顺序如果影响玩法，必须用 `.chain()`、`.before()` 或
`.after()` 显式排序；统一队列保证既有入队顺序，不替应用决定生产顺序。

## 帧输入与 FixedUpdate

设备输入通常按帧更新，固定 tick 可能在一帧内运行零次、一次或多次。不要在每个
`FixedUpdate` 直接重复读取帧级 `just_pressed` 作为一次性触发：

1. 帧输入采集系统在设备状态更新后记录逻辑动作，按明确顺序写入游戏自己的缓冲。
2. `RequestProducers` 消费尚未处理的输入，每条输入只提交一次。
3. 没有 fixed tick 的帧保留缓冲；同一帧后续 fixed tick 不重复提交已消费输入。

缓冲可以是玩家 Component 或有序 Resource，具体存储由应用决定。示例在 `PreUpdate` 解析
按下时的 Handle 并放入 FIFO，在 `FixedUpdate` 消费；等待期间修改槽位不会让旧输入改为施放
新技能。如果游戏选择缓冲动作并在 fixed tick 解析，则使用消费时的绑定，应明确这一行为差异。
网络同步或回放还应由游戏为输入分配明确的 tick 和稳定玩家顺序。

长按连发、蓄力开始/释放、输入容错窗口和 UI 焦点过滤也属于游戏输入层。需要将释放传给已经
开始的技能时，应建立明确的事件或请求通道；更改绑定配置本身不会通知运行中的技能。

## 技能移除与 ASC 生命周期

`AbilitySpecHandle` 是 ASC 局部编号，不能跨角色复制使用。即使两个角色 Handle 数值相同，
也可能指向不同技能；`resolve()` 只能验证所提供 ASC 中是否存在该编号，无法识别来自别的 ASC
的同值 Handle。因此必须保持“绑定与拥有技能的 ASC 位于同一实体”的调用约定。

成功移除规格之后，再清理它的全部输入别名：

```rust
if ability_system.clear_ability(handle) {
    bindings.unbind_ability(handle);
}
```

`clear_ability()` 在规格仍活跃或 Handle 不存在时返回 `false`。如果游戏的意图只是清空技能栏，
直接调用 `unbind()` 即可，不必删除角色已学会的技能。若漏做成功移除后的绑定清理，
`resolve()` 会返回 `AbilityNotGranted`；ASC 不反向查询输入组件。

替换整个 ASC、转移控制对象或复制角色配置时，必须清空并针对新的技能授予结果重建绑定。
当前 Handle 不包含实体身份，不应持久化为跨会话的技能标识；保存技能栏时由游戏记录稳定技能
定义标识，并在加载、授予完成后转换成新 Handle。

## 迁移与验证

旧的 `bevy_gas::gas::ability_input`、GAS prelude 或 crate-root 输入绑定导入，统一迁移到：

```rust
use bevy_gas_template::gameplay::ability_input::{
    AbilityInputBindingError, AbilityInputBindings,
};
```

类型名称、错误变体、绑定顺序和解析行为保持不变。GAS 不依赖模板，也不保留旧路径重导出。
授予技能后再建立绑定（外围函数返回可接收 `AbilityGrantError` 的 `Result`）：

```rust
let handle = ability_system.give_ability(ability, level)?;
let _ = bindings.bind(PlayerAction::Slot(0), handle);
```

直接构造规格时使用 `GameplayAbilitySpec::new(handle, ability, level)`，输入字段相关 getter
和 setter 已移除。原先匹配带编号 `Input` 原因的代码改为匹配
`AbilityActivationReason::Input`，动作信息由输入层保存。

运行 [`examples/ability_input_bindings.rs`](../examples/ability_input_bindings.rs) 查看完整接入：

```bash
cargo run --example ability_input_bindings
```

示例使用无窗口的输入模拟，展示一次按住 Q 跨越多个 fixed tick 只触发一次，以及重绑后再次按下
会使用新技能。以上命令在模板仓库根目录运行。`src/gameplay/ability_input.rs` 是门面，
`ability_input/bindings.rs` 和 `ability_input/error.rs` 分别拥有组件行为和错误定义。
测试位于 `tests/gameplay_test/ability_input_test.rs`，由 `tests/gameplay_test.rs` 加载：

```bash
cargo test --test gameplay_test ability_input_test
```

## 独立的空间随机流

`SpatialRandom` 封装独立的 `rand::rngs::StdRng`，作为 Bevy `Resource` 按需由应用初始化。
GAS `RandomPlugin` 仍只安装 `bevy_gas::Random`；空间随机不访问或借用该资源。

```rust
use bevy::prelude::*;
use bevy_gas::Random;
use bevy_gas_template::gameplay::randoms::SpatialRandom;

fn configure_random_streams(app: &mut App) {
    app.insert_resource(Random::from_seed(100));
    app.insert_resource(SpatialRandom::from_seed(200));
}
```

可使用 `app.init_resource::<SpatialRandom>()` 采用默认种子 `123456`。上例显式指定各自种子，
调用方应将它们纳入对局初始化或回放数据；相同种子并不意味着两个资源共享状态。

| API | 参数、返回值与行为 |
| --- | --- |
| `SpatialRandom::DEFAULT_SEED` | 默认种子 `123456` |
| `SpatialRandom::from_seed(seed)` | 由 `u64` 种子构造独立随机源 |
| `set_seed(seed)` | 重置空间随机序列，不修改 GAS 概率随机流 |
| `from_rng<T>()` | 返回实现 Bevy `FromRng` 且支持标准分布的随机值 |
| `sample_interior(shape)` | 接收 `ShapeSample` 形状引用，返回局部坐标中的内部采样点 |
| `sample_boundary(shape)` | 接收 `ShapeSample` 形状引用，返回局部坐标中的边界采样点 |

Bevy 0.20 的采样接口使用 `bevy::math::FromRng` 和 `bevy::shape::ShapeSample`。
模板直接启用同版本官方 `bevy_shape/rand`，复用现有 `rand` 0.10.2；GAS 移除对
`bevy_shape` 的直接依赖。本次没有引入其他第三方库或版本。

旧 `Random::from_rng`、`sample_interior`、`sample_boundary` 调用迁至 `SpatialRandom`，
对应系统参数改为 `ResMut<SpatialRandom>`。GAS `Random` 继续提供种子、范围采样和概率采样，
并负责效果概率。空间采样次数不会推进 GAS 概率随机流，效果掷骰也不会推进空间随机流。
这有意改变了旧代码把空间与效果混用一个随机流时的后续序列；依赖旧混合调用序列的回放应更新版本。

同一依赖版本、种子与调用顺序下可复现序列；不承诺跨依赖版本或跨平台浮点位级一致。
多个系统共享 `SpatialRandom` 时仍需明确排序。表现随机若会随帧率改变调用次数，应再使用
游戏自行拥有的独立状态，不与逻辑空间随机混用。

源码由 `src/gameplay/randoms.rs` 门面和同名目录维护，测试覆盖相同种子重放、重置、
几何采样边界及两个资源的流隔离：

```bash
cargo test --test gameplay_test spatial_random_test
```

## 当前扩展边界

这次只迁移已有输入绑定和几何采样能力。成本与冷却仍在技能启动前提交，原生任务仍为
`Instant` / `WaitTicks`；没有新增延迟提交、外部信号等待、连招、动画或战斗规则模块。
后续有实际需求时，由 GAS 提供通用提交/任务协议，游戏决定触发时机与具体动作规则。
