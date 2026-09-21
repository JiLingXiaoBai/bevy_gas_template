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

## 首次工具准备与运行

当前固定 Luban 安装与导表流程支持 Windows x64，需要 Rust edition 2024 工具链、Git、
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
