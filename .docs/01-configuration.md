# 配置所有权、加载与扩展

## 两个项目的边界

`bevy_gas_template` 拥有 Excel、XML Schema、生成 Rust 类型、二进制包、加载器、业务校验、配置编译器和 Catalog。
游戏代码通过本包的 `config` 访问这些类型，通过 `bevy_gas` 访问 GAS 公共类型。
GAS 核心无须知道游戏的 Excel 字段、配置 ID、空间索引或表现方式。
本包 `gameplay::targeting` 拥有索敌定义和队列；编译器构造这些游戏类型，再将结果交给 GAS。

配置模块已完整归属游戏，使用 `bevy_gas_template::config`（游戏包内部可使用 `crate::config`）与
本游戏的 `config::compile_catalog`。当前本地 GAS 已移除配置模块、Excel 和导表工具；配置完整归属游戏。

当前 Cargo 使用 `bevy_gas = { path = "../bevy_gas" }`。模板或游戏与 GAS 库默认放在同级；
游戏目录名可以变化，只要该相对路径仍指向库的 Cargo.toml。使用者需另行取得 GAS 源码，或调整 path。
本地库改动会参与下次构建，不需要提交或推送；本地联调与可选 Git 依赖见
[04 — 模板使用与依赖维护](04-template-and-dependencies.md)。

## 启动流程

基础启动器通过 `application::SimulationPlugin` 装配模拟，再注册自己的 `game::GamePlugin`，默认不加载配置。配置演示的启动与场景位于可删除的 `examples/configured_game/`，它组合 `config`、`gameplay` 和共享模拟。
可复用的 `gameplay` 实现不读取示例配置或创建 Laser 场景；配置编译器仍只引用纯索敌定义。
详见 [07 — 架构与插件组合](07-architecture-and-composition.md)。

需要配置的游戏可按下面的顺序接入；模板的 GamePlugin 扩展方式见 [09 — 从模板开始新游戏](09-starting-a-game.md)。

1. 确认 SimulationPlugin 已安装（基础 main 已完成，包含 GAS）；需要技能命令时添加 AbilityCommandPlugin，它会补充 TargetingPlugin；只需索敌时单独添加 TargetingPlugin，需要表现时另装 PositionPresentationPlugin。
2. 使用游戏 load_tables 读取自己生成的八张表。
3. 使用游戏 compile_catalog 校验引用、数值和动作载荷，建立共享 GAS 定义和游戏 TargetingDefinition。
4. 将游戏 GameplayCatalog 插入 World。
5. 创建 GameplayAbilitySystemBundle，初始化属性，以游戏 grant_ability 授予技能并保存 ID 到 Handle 的映射。
6. 将目标请求放入游戏 TargetingRequestQueue，确认目标后写入 GAS GameplayExecutionQueue；
   已知目标也可直接提交激活或效果请求。完整空间与调度边界见 [05 — 空间索敌](05-spatial-targeting.md)。

Catalog 构建保留原实现的失败原子性：克隆 UniqueNamePool、GameplayTagManager、AttributeIdManager，
在快照中完成注册与全部定义构造，成功后才提交。属性区域和既存标签继承冲突均返回错误。
当前依赖库公开的 register_tag_internal/register_id_internal 可供编译层在注册表快照上工作；
名称中的 internal 不代表 Rust 的私有可见性。后续升级依赖时需验证这些公共接口的兼容性。

同一 EffectId 复用一个 Arc，不能因不同技能引用而重复构造同身份效果。内部标签位、属性索引、技能 Handle
只在当前 World 有意义，存档和配置使用稳定 ID 或名称。注册与动作执行保持明确顺序，时间单位为固定 tick。

## 导表与部署

输入为 `config/tables/`、`config/defines/`、`config/luban.conf` 和 `config/templates/`。
手写逻辑位于 `src/config/`，不要写入生成目录。

`config/export.ps1` 使用本项目的固定 Luban 5.0.0 工具，先暂存生成 Rust 和二进制，再编译本游戏的
`gas-config` 校验程序。暂存 Cargo 清单把本地 GAS 路径转为绝对路径，仍引用同一份库源码，
不复制库源码，根清单保持相对 path；如果以后选择 Git 依赖，暂存清单保留其 URL 与 rev。
校验构建使用独立的 `target/config-export/`，与游戏的 `target/debug/` 隔离，避免临时工程的编译时路径污染正常游戏产物。首次需要编译这套校验依赖，后续导表复用它。清单生成与完整业务校验通过后，发布 `config/generated/` 和 `assets/config/`；失败按原流程回滚。
`config-validation` 只属于本模板配置层；当前本地 GAS 不再定义该 feature。

新生成的 Rust 模块和数据始终配套。运行时默认读取 manifest
并验证结构摘要与内容摘要；`--no-default-features` 关闭后只加载 .bytes。二进制资产由导表生成且 Git 忽略。
当前导表支持 Windows x64；首次克隆后先准备同级 GAS，再执行
`tools/luban/setup.ps1`，再运行 `cargo fetch --locked` 缓存 Rust 依赖，随后导表并 `cargo run --example configured_game` 运行配置演示；基础 `cargo run` 无需导表，日常导表保留 offline/locked 策略。

## 新增类型与字段

- 只改数值或已有动作组合：编辑游戏 Excel，导表并验证。
- 新增字段或表：修改游戏 Schema/表头、对应游戏 compiler/校验/索引，重新导表，更新测试。
- 自定义数值公式：游戏实现 ModifierMagnitudeCalculation，编译器从表行构造
  ModifierMagnitude::Calculated。上下文仅提供库公开的只读能力，不能隐式访问任意游戏 ECS 状态。
  `source_attribute(id)`／`target_attribute(id)` 读取求值时的来源／目标属性（含尚未重算的修改器），
  `source_snapshot()` 则读取调用方显式捕获的历史快照。效果在应用时求值，周期执行继续复用结果。
  公式返回类型仍为 f32；非有限修改器幅度由 GAS 以 NonFiniteModifier 错误拒绝，不能用零掩盖溢出。
  本模板 LinearLevel 保留这项运行时检查，完整制作校验另检查技能允许等级范围。
- 游戏事件动作：运行时已有 EmitEvent，但当前示例表只编译 ApplyEffect/EndAbility；需要先增加游戏配置映射。
  Observer 延迟执行，不能把它当作启动阶段同步任务。弹体的取消与持久化规则由游戏定义。
- 新的原生任务生命周期或基础聚合运算：先在 GAS 库实现并测试，再通过依赖升级接入。

配置编译器构造 GAS 库公开的任务与 Modifier 定义，游戏在本包内维护配置映射与额外玩法。

## 验证范围

配置集成测试使用本游戏自己的生成类型与 compiler，覆盖注册失败保持、引用顺序、数据解码、
额外成本、授予/撤销和校验 feature。`examples/configured_game/` 的无窗口验收读取本项目真实导表产物，再调用外部 GAS
执行示例技能。示例的窗口和无窗口路径共享场景初始化与状态读取，专用测试通过 `cargo test --examples` 执行；基础测试不依赖示例目录。

完整工具链与 MCP 约定见 [02 — Luban 工具链](02-luban-toolchain.md)，表字段、编译规则与错误处理见
[03 — Excel 技能配置与 GAS 接入](03-gas-configuration.md)。所有测试入口及运行命令见项目 README。改变样例的数值与时间线后，应同时更新对应验收预期。
