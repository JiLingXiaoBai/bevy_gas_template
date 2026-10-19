# 07 — 架构与插件组合

## 模块所有权

两个包保持单向依赖：游戏模板依赖 GAS，GAS 不依赖模板。模板内部按职责组合：

```text
application → config → gameplay::targeting 的纯定义
            → gameplay 的可选插件和组件
            → bevy_gas
config      → bevy_gas 的定义与注册接口
gameplay    → bevy_gas 的公开契约和阶段
```

`application` 拥有 Laser 演示场景、默认配置路径、启动和验收逻辑。`gameplay` 的实现只提供可复用
能力，不读取演示配置、不初始化演示场景。`config` 构造 GAS 定义和游戏索敌定义，不启动应用或插件。

新代码使用 `bevy_gas_template::application::{run_headless, run_windowed, default_config_directory,
GameResult, LaserReport}`。原 `gameplay` 路径显式重导出这五项作为兼容入口，内部实现和主程序使用
新的规范路径。兼容门面不承担应用实现，也不要求使用者修改既有公共函数名。

## 独立的游戏调度

`GameplaySet` 只描述阶段名称；`GameplaySchedulePlugin` 注册阶段顺序，不创建索敌队列、索引、
渲染或 GAS 运行时资源。需要移动但不需要索敌时，可显式组合：

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

窗口实现单独编译；无窗口运行不会仅因链接模板而需要渲染、音频或窗口后端。
`run_windowed` 在未启用功能时返回明确错误，CLI 仍由 `--headless` 显式选择运行模式。
配置读取与编译、逻辑位置、Transform 插值、索敌和 GAS 执行在基础构建中可用。
导表脚本使用无窗口的配置校验构建，不改变生成格式或发布流程。

新增游戏功能优先放入所属领域、通过独立 Component/Resource/System/Plugin 组合；演示装配放在
application。不要让可复用能力为了获取示例实体或配置路径而反向依赖应用。

## 验证

既有技能、效果、输入、空间与配置测试继续覆盖默认行为。调度回归验证不安装索敌时的移动与
Commands 可见性、Targeting 自动安装和显式前置安装。无窗口测试验证入口错误和配置技能的真实
执行结果。默认与最小构建均检查 Clippy 和测试；`cargo tree --no-default-features --edges normal`
可检查运行时依赖，测试专用的 keyboard feature 不改变发布库的渲染依赖边界。
