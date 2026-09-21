# AGENTS.md — bevy_gas_template

这是独立游戏模板，使用 Rust edition 2024、Bevy 0.19.1，默认通过 `bevy_gas = { path = "../bevy_gas" }` 使用同级本地库。
模板或游戏目录可任意命名，但默认布局要求与 `bevy_gas` 同级；本地库改动在下次构建生效，无需推送。
游戏拥有 `config/`、`src/config/` 与技能配置，GAS 核心源码保留在依赖仓库。

- 正确性、可读性、性能依次优先；不新增无必要的第三方依赖，不使用 unsafe。
- 运行时代码不得使用 unwrap、expect、panic；错误用 Result 传播，测试可使用明确成功断言。
- 所有代码注释与公开 API 文档使用英文；知识库尽量使用中文。
- 状态以 Bevy Component/Resource 为中心，系统明确排序；Gameplay 时间用 FixedUpdate tick。
- 新模块使用门面文件与同名目录；显式 pub use，禁止通配公开重导出。
- 游戏配置使用本包 `config`，GAS 类型从 `bevy_gas` 导入；当前本地 GAS 已移除配置层，游戏配置由本包独立维护。
- 默认保留本地 path 依赖；不要未经用户要求改成 Git。GitHub 模板使用者需另行取得同级 GAS，或自行调整 path。
- 只有用户选择 Git 依赖时才要求真实已推送的固定提交；本地开发不要求提交或推送。依赖变化后同步锁文件并完成回归。
- 当前导表支持 Windows x64；首次确认同级 GAS 存在，再执行 setup.ps1、cargo fetch --locked、config/export.ps1 和游戏。
- GitHub 模板不会自动同步后续修改或替换 crate 名；改名时同步 Cargo package/default-run、源码/测试导入、文档和技能上下文。
- Excel/Schema 修改后运行 `pwsh -NoProfile -File config/export.ps1`；不要手改生成代码或清单。
- 导表生成 `config/generated/` 和 `assets/config/`；使用本项目的校验程序和固定 Luban 工具链。
- 集成测试按领域放入 `tests/config_test/`、`tests/gameplay_test/`，由同名顶层测试门面加载。
- 修改代码后及时更新 `.docs/`；只做与需求有关的修改。
- 完成后运行 cargo fmt、cargo clippy --all-targets --all-features -- -D warnings、cargo test、cargo test --all-features、cargo build，以及 cargo run -- --headless。
