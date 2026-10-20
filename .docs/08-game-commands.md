# 08 — 游戏命令与技能结果关联

## 所有权与范围

`gameplay::commands` 提供两层能力：

| API | 职责 |
| --- | --- |
| `GameCommandQueue<C>` | 游戏自定义命令的跨帧 FIFO、checked 递增编号 |
| `AbilityCommand` / `AbilityCommandPlugin` | 把技能意图接入现有索敌与 GAS 队列，并返回关联结果 |

泛型队列不安装插件，不依赖索敌执行，也不把 Move、Build 等游戏命令解释成技能。
RTS 单位命令、ACT 蓄力开始/释放可以使用自己的枚举和消费系统；需要激活技能时，再产生
`AbilityCommand`。技能消耗、冷却、标签与任务仍由 GAS 负责。

## 安装与提交

```rust
use bevy::prelude::*;
use bevy_gas::GameplayAbilitySystemPlugin;
use bevy_gas_template::gameplay::commands::AbilityCommandPlugin;

fn configure(app: &mut App) {
    app.add_plugins((GameplayAbilitySystemPlugin, AbilityCommandPlugin));
}
```

上面展示自行创建 App 的低层组合。基础 main 已经通过 SimulationPlugin 安装 GAS，在
`GamePlugin` 中只追加 `AbilityCommandPlugin`，不要再次安装 GAS。

`AbilityCommandPlugin` 会在未安装时补充 `TargetingPlugin`，后者补充独立游戏调度插件。
若显式列出这些插件，应按 Schedule、Targeting、AbilityCommand 的顺序安装，不要重复添加
Bevy 唯一插件。通用命令队列本身不需要上述插件。

```rust
use bevy::prelude::*;
use bevy_gas::AbilitySpecHandle;
use bevy_gas_template::gameplay::commands::{
    AbilityCommand, GameCommandId, GameCommandQueue, GameCommandQueueError,
};

fn submit(
    source: Entity,
    handle: AbilitySpecHandle,
    target: Entity,
    queue: &mut GameCommandQueue<AbilityCommand>,
) -> Result<GameCommandId, GameCommandQueueError> {
    queue.push(AbilityCommand::direct(source, handle, target))
}
```

- `AbilityCommand::direct(source, handle, targets)` 接收已解析的目标；`targets` 可以是 Entity
  或 GAS `AbilityActivationTargets`，不额外进行空间与敌我判定。
- `AbilityCommand::targeted(source, handle, input, definition)` 在当前 fixed tick 的空间同步后
  执行索敌。`input` 是 `TargetingInput`，`definition` 是 `Arc<TargetingDefinition>`。
- 两种构造方式默认产生 Input 原因。`with_origin(AbilityCommandOrigin::Direct)` 供直接游戏逻辑使用；
  构造方法中的 `direct` 指目标已选好，不表示激活原因。
- Handle 与实体在构造时捕获；索敌 origin、direction 也在构造时捕获，候选目标来自执行时的逻辑世界。
  `examples/configured_game/` 的窗口演示在输入帧从 `LogicalPosition` 捕获原点，演示无窗口验收在首次 tick 前捕获；
  两者都不把插值后的表现位置带回模拟。需要先移动再捕获原点的游戏应在
  `RequestProducers` 构造技能命令，或先缓冲游戏动作、到该阶段再转换成技能意图。
- 生产者不需要访问 GAS 队列来创建 chain。适配器在提交时分配 root chain。链式技能、复杂 instigator/
  causer 或属性快照继续使用已有 GAS 低层 API，本层不复制整套激活上下文。

## 调度与 FIFO

```text
GAS RequestProducers
→ AbilityCommandSet::Prepare：一次性消费游戏命令，记录原顺序并提交索敌
→ GameplaySet::SpatialSync
→ GameplaySet::Targeting：发布 buffered 索敌结果
→ AbilityCommandSet::Submit：按原命令顺序向 GAS 提交所有成功命令
→ GAS PreGameplayConvergence / GameplayResolve / 后处理 / RecalculateAttributes
→ AbilityCommandSet::Results：按原命令顺序发布终态结果
```

即使较早命令需要索敌、较晚命令已有直接目标，后者也不会越过前者进入 GAS 执行队列。
索敌失败的命令返回错误，后续命令仍继续。这里保证的是本命令适配器内的 FIFO；
直接向 GAS/索敌队列提交的其他生产系统仍遵守各自入队顺序，游戏必须明确它们的调度关系。

设备输入可以按帧先缓冲逻辑动作，AI、脚本与 UI 也可以写入命令队列。没有 fixed tick 的帧保留命令，
补帧时不会重复提交已经消费的命令。需要当前 tick 执行的生产者应放在 `RequestProducers` 或其他
明确早于 `AbilityCommandSet::Prepare` 的位置。Prepare 之后产生的命令在下一 tick 消费。
读取最终状态和结果的 fixed 系统应排在 `AbilityCommandSet::Results` 之后。

## 结果与关联

`AbilityCommandResult` 是 Bevy Message，可用 `MessageReader<AbilityCommandResult>` 读取：

| 字段 | 含义 |
| --- | --- |
| `command_id` | 游戏命令队列返回的 `GameCommandId` |
| `source` | 技能所属 Actor |
| `targeting_request_id` | 索敌请求 ID；已有目标时为空 |
| `execution_request_id` | 被 GAS 接受的请求 ID；索敌/提交失败时为空 |
| `outcome` | 当前 root 激活请求的终态 |

`AbilityCommandOutcome` 区分：

- `Succeeded`：GAS 主激活成功；不代表技能已结束，也不承诺所有 best-effort 启动动作都成功。
- `TargetingFailed(TargetingError)`：包括没有目标、无效输入、缺失位置等，尚未进入 GAS。
- `SubmissionFailed(AbilityCommandSubmissionError)`：无法构造激活目标、GAS 队列拒绝提交，或下游请求
  被清理/系统未运行而没有收到预期结果。
- `Rejected(GameplayExecutionError)`：冷却、成本、重复实例等正常玩法拒绝。
- `Failed(GameplayExecutionError)`：缺失 ECS 状态或无效配置等执行失败。

已有 `TargetingResultEvent` API 和 Observer 保留。新增 `TargetingRequestResult` Message 在索敌系统内
直接写入，可在 Targeting 后、resolver 前消费；不依赖 Observer 的延迟执行。Message 同时公开旧
`ActivateAbility` continuation 的提交结果与被接受的 GAS request ID，提交失败不再只有日志。
AbilityCommand 使用 `EmitResult`，由自己的 Submit 阶段统一提交，从而保持混合命令的 FIFO。

结果按 Bevy Message 保留周期存储；UI/业务消费者需每帧或每 tick 读取。若要长期保存回放与审计，
由游戏在消费时写入自己的历史，不应依赖 Message 无限保留。

## 生命周期与标识边界

`GameCommandId` 在某个队列实例内单调递增；溢出返回 `IdExhausted`，不追加命令、不复用编号。
`clear()` 只清理仍在泛型队列中的命令，不发布取消结果，也不影响 Prepare 已消费的命令；
`pop()` 则将执行与反馈责任转交调用方。需要显式取消语义的游戏应自行定义取消命令。

不要在适配器的 Prepare/Submit/Results 之间清除它已提交的下游请求。如果游戏确实清除了索敌或 GAS
队列，适配器会在本 tick 返回 `MissingTargetingResult` / `MissingExecutionResult`，释放内部待处理记录，
不会永久等待。未安装 GAS 或依赖资源属于插件组合错误，应在装配时修复。
暂停应统一门控整条模拟管线，而不是只停 resolver、继续运行命令 Submit/Results。
`MissingExecutionResult` 只说明当前 tick 未观察到结果，不会撤回仍留在 GAS 队列中的请求；
部分暂停后恢复 resolver 可能继续执行这些请求。

命令 ID、索敌 ID、GAS request ID 均是本 World/队列生命周期内的关联标识，不是跨存档、跨进程身份。
旧索敌队列仍保留既有 wrapping 编号行为。本层没有新增网络协议、回滚状态、稳定实体 ID 或跨平台
浮点确定性保证；联网/回放应由游戏定义 tick、玩家顺序、持久实体映射及数值规则。

## 验证

`tests/gameplay_test/commands_test.rs` 覆盖独立泛型 FIFO、混合直接/索敌命令顺序、无 fixed tick 的帧、
只消费一次、输入/直接激活原因、索敌失败后的继续执行、GAS 拒绝/失败、外部清理下游队列后的终结，
以及旧索敌 continuation 的执行 ID 反馈。队列模块的单元测试覆盖编号耗尽不改变队列。
