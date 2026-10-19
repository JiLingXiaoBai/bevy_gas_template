# 04 — 模板使用与依赖维护

## 发布为 GitHub 模板

将本工程上传到你管理的 GitHub 仓库，并提交 Cargo.toml、Cargo.lock、源码、测试、Excel、Schema、
生成 Rust 模块、工具脚本、锁文件、文档、技能与许可。保留 `.gitignore`：`target/`、工具缓存与
`assets/config/` 中的导表数据不作为模板输入，使用者首次导表重新生成二进制资产。

上传后进入仓库 Settings，勾选 Template repository。该设置需要仓库管理员权限，
仅修改本地目录名不会打开 GitHub 的模板功能。[GitHub 官方设置说明](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-template-repository)

本项目的 `publish = false` 是 Cargo 的包发布设置，不影响将 Git 仓库上传到 GitHub 并设为模板。
默认 path 依赖可以保留；模板使用者必须另外取得兼容的 GAS 源码，按下面的目录约定摆放或自行调整 path。

## 创建新的游戏

在模板仓库选择 Use this template → Create a new repository，填写新仓库的所有者、名称和可见性，
然后克隆新仓库，并另行准备同级的 bevy_gas。GitHub 按模板复制文件并为新项目建立独立历史；模板后续变化不会自动合入游戏，
需要游戏自行比较、选择并验证所需修改。[GitHub 官方使用说明](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-repository-from-a-template)

仓库名、文件夹名和 Rust 包名可以不同。GitHub 的模板创建只复制文件，本模板没有自动替换名称的初始化脚本，
因此新项目可以先沿用 `bevy_gas_template` 编译运行。若改为自己的 Rust 名称，例如 `my_game`：

1. 修改 Cargo.toml 的 `package.name` 与 `package.default-run`。
2. 同步 `src/`、`tests/` 中的 `bevy_gas_template` crate 导入，以及 README、AGENTS、`.docs/` 和
   `.agents/skills/` 中的项目名称与上下文。依赖库的 `bevy_gas` 名称保持不变。
3. 导表程序的 `gas-config` 名称由 export.ps1 使用，改游戏名时保留该名称。
4. 运行 `cargo check` 让 Cargo 更新包名对应的 Cargo.lock，并提交更新后的锁文件；不要手改锁文件。
   然后执行下面的首次运行流程与回归检查。

## 本地库目录约定

当前默认依赖：

```toml
bevy_gas = { path = "../bevy_gas" }
```

模板或游戏与库放在同一父目录，例如：

```text
workspace/
├── bevy_gas/
└── my_game/    # The template or any game created from it.
```

my_game 可以替换为 bevy_gas_template 或任意游戏目录名，保持同级就不必修改默认 path。
请另行取得兼容的 GAS 源码；仅克隆模板不会自动下载这个 path 依赖。如果库位于其他目录，
修改 path 指向实际包含 GAS Cargo.toml 的目录。path 始终相对于游戏自己的 Cargo.toml。
[Cargo 本地依赖文档](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-path-dependencies)

## Bevy 0.20 升级基线

模板和同级 GAS 库统一使用 Bevy 0.20.0，最低要求 Rust 1.97.1，edition 仍为 2024。
最低工具链版本来自 [Bevy 0.20.0 Cargo 元数据](https://docs.rs/crate/bevy/0.20.0/source/Cargo.toml)。
更新两个项目的 Bevy 依赖后，应分别由 Cargo 更新锁文件，并按下面的本地联调流程验证；
保持 `bevy_gas = { path = "../bevy_gas" }`，无需切换到 Git 依赖。

GAS 的生命周期 Observer 已适配 `On<Discard<T>>`，旧的 `On<Discard, T>` 写法不再使用；
`Add` 和 `Remove` 的组件泛型也移入事件类型。模板仍按配置建立技能，请求生产系统仍在既有 `FixedUpdate`
阶段生产请求，GAS 的 FIFO、同 tick 结算和 startup 同步执行契约保持不变。详细说明见
[GAS 插件与生命周期](../../bevy_gas/.docs/02-plugins-and-lifecycle.md#bevy-020-升级基线)。

现有模板的 `Camera2d`、`Sprite::from_color` 可继续使用；HUD 的 `TextFont` 已显式指定
`FontSize::Px`，不依赖字体大小的默认单位。本次引擎升级不调整游戏配置格式或技能数值。

## 首次工具准备与运行

当前固定 Luban 安装与导表流程支持 Windows x64，需要 Rust 1.97.1+（edition 2024）工具链、Git、
PowerShell 7.2+、系统 Microsoft.NETCore.App 8.0.0+ 正式版，以及 PATH 中的 7z.exe 或 7za.exe。
其他平台尚未配置对应的工具安装与导表流程。首次需要网络获取固定版本工具和锁定的 Cargo 依赖。

先确认 ../bevy_gas/Cargo.toml 存在，再在克隆后的模板或新游戏根目录依次运行：

```powershell
pwsh -NoProfile -File tools/luban/setup.ps1
cargo fetch --locked
pwsh -NoProfile -File config/export.ps1
cargo run
```

Cargo 直接构建 path 指向的本地库；cargo fetch 只负责缓存其他需要取得的依赖，不会下载该本地库。
本项目工具按自身脚本位置读取配置，
缓存位于 `tools/luban/.cache/`。导表校验使用独立 `target/config-export/`，发布
`config/generated/` 和 `assets/config/`。所有配置、工具及脚本均由新游戏拥有。

日常仅改表时重新执行 export.ps1，再运行游戏或 `cargo run -- --headless`；完整检查命令见项目 README。
导表采用 offline/locked 构建，依赖未缓存时应先完成 `cargo fetch --locked`。

## 窗口与无窗口依赖

默认开启 `windowed` 和 `config-validation`，原窗口体验与校验行为保持不变。
`windowed` 启用 Bevy 原有默认功能；基础依赖只保留 ECS、数学、Transform、时间和日志等能力，
位置插值并不需要 GPU，因此在无窗口构建中仍可使用。

```powershell
cargo run --no-default-features -- --headless
cargo run --no-default-features --features config-validation -- --headless
cargo run --no-default-features --features config-validation --bin gas-config -- validate-config assets/config
```

第一条同时关闭完整配置校验；后两条保留校验。未启用 `windowed` 时调用窗口入口会返回明确错误，
CLI 应显式使用 `--headless`。导表暂存工程同样只启用 `config-validation`，不编译窗口和渲染。
原 `gameplay` 下的应用入口保持兼容导出，新代码从 `application` 导入，见
[07 — 架构与插件组合](07-architecture-and-composition.md)。

## 本地联调

当前已使用本地 path 依赖。修改同级 bevy_gas 的源码后，下次构建游戏或导表就会使用修改后的代码，
不需要先提交或推送。当前本地 GAS 已移除配置层，模板使用自己的 bevy_gas_template::config，
Excel、生成类型、Luban 工具和配置包始终由游戏维护。

export.ps1 通过 Cargo metadata 读取本地库路径，只在暂存 Cargo.toml 中把 path 转成绝对路径，
根清单保持相对路径，不复制 GAS 源码。当前导表脚本支持 bevy_gas 作为唯一的本地 path 依赖。
Cargo.lock 不记录本地库的源码提交，团队需要自行约定使用的库版本。

库的公共 API、依赖或位置变化后，先运行 cargo check 检查并按需更新锁文件，再运行完整导表，
以及 README 中的 fmt、clippy、默认和全部 feature 测试、build 与无窗口验收。
配置编译规则有变化时，同步更新游戏适配层、生成结果与文档。本地使用不要求更换为 Git 依赖。

模板自身更新和 GAS 库改动分别维护：新模板文件不会自动合入已创建的游戏，
修改 GAS 也不会自动更新游戏的 Excel、工具脚本或配置编译器。

## 空间与索敌迁移

当前模板接收原 GAS 库中的逻辑位置、动态 AABB 树、索敌定义/队列和可选表现。
GAS 只保留 `AbilityTargetData` / `AbilityTargetHit` / `AbilityActivationTargets` 契约，
默认与自定义成本插件组均不再安装空间索敌或表现。

游戏需同步调整导入，显式安装 `TargetingPlugin` / 所需的 `PositionPresentationPlugin`，
将三个空间阶段改为游戏 `GameplaySet`；配置编译器引用本包索敌定义。
本次迁移不改变 Excel、Schema 或二进制包结构，不新增第三方依赖，不恢复扫描后备，
也不把原有点索敌改成体积碰撞。模块和测试迁移清单见
[05 — 空间、索敌与 GAS 接入](05-spatial-targeting.md)。

## 输入绑定与空间随机迁移

GAS 已移除 `ability_input` 领域与所有输入绑定重导出。游戏改从本包
`gameplay::ability_input` 导入 `AbilityInputBindings` 和 `AbilityInputBindingError`；
示例与测试随之移入本模板，名称和绑定行为保持不变。

`Random` 的 `from_rng`、`sample_interior`、`sample_boundary` 迁移到本包
`gameplay::randoms::SpatialRandom`，使用独立 `StdRng`，由游戏按需显式初始化。
官方 `bevy_shape/rand` 直接依赖从 GAS 转到模板，复用已有 `rand` 0.10.2，
不引入新的第三方库或版本。两库分别更新 Cargo.lock 并完成回归。

空间采样不再消耗 GAS 概率序列；旧混合随机流的回放应明确版本变化。
本次不调整技能提交时机，不新增任务类型或具体游戏动作流程。
完整迁移和验证命令见 [06 — 输入绑定与空间随机](06-input-and-spatial-random.md)。

## 可选的固定 Git 依赖

如果以后希望由 Cargo 自动获取 GAS、减少同级目录要求，可以主动改用 Git 依赖。
这是一种可选方式，当前默认仍是本地 path。

```toml
# Replace the revision with a real, already-pushed commit.
bevy_gas = { git = "https://github.com/JiLingXiaoBai/bevy_gas.git", rev = "<full-commit-hash>" }
```

使用时把示例中的占位值替换为远端真实存在且与游戏兼容的完整提交哈希，同时移除 path；
git 和 path 不能同时用于同一个依赖声明。然后运行 cargo check 更新解析和 Cargo.lock，
执行 cargo fetch --locked、完整导表及游戏回归验证。暂存导表项目会保留 Git URL 与 rev。
[Cargo Git 依赖文档](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories)

只有选择 Git 依赖时，所需库改动才必须在所引用的远端提交中。固定完整哈希后，仅运行
cargo update 不会越过该 rev 跟随默认分支；升级时显式修改 rev，并更新 Cargo.lock 与验证结果。
保留本地 path 依赖同样可以发布和使用 GitHub 模板，只需说明库的取得方式与目录约定。
