# agent.md — 本仓库 Agent 与 Git 工作规范

> 本仓库由一名维护者开发，是 PhotoCraft 的跨平台适配仓库，主要面向 iOS / Android / macOS。
> 本文件规定本仓库的产品边界、Git 工作流、提交格式、跨平台验证和发布方式。
> 核心目标：共享一套代码，`main` 始终保持可构建、可验证、可发布，平台差异由代码和构建配置表达，而不是由长期分支表达。

## 1. 产品与仓库边界

- iOS 与 Android 应用统一叫 **PhotoComfy**，机器名使用 `photocomfy`；macOS 桌面端沿用 **PhotoCraft**。
- 保留历史路径 `apps/photocraft-ios/`、`apps/photocraft-android/` 以及现有 crate 名，不为了产品命名而重命名它们。
- 共享 Rust 核心位于 `crates/`，随上游演进；平台适配优先放在 `apps/`、平台胶水层和平台构建目录中，改动应尽量小。
- 本仓库不为上游 PhotoCraft 核心功能本身额外补充或修复测试；但修改共享代码时，必须运行受影响的现有测试，并保证本仓库的平台适配能够继续构建和运行。
- 平台功能应优先复用共享引擎和 UI。不得通过复制整套平台代码、建立平台私有实现或长期分支来规避共享代码修改。

## 2. Git 核心原则

- `main` 是唯一的长期分支，也是远程仓库的默认分支。
- 不建立长期存在的 `develop`、`release`、`ios-dev`、`android-dev`、`macos-dev`、平台分支或 hotfix 分支。
- 小型、低风险且已完成验证的改动可以直接提交到本地 `main`，验证后推送。
- 高风险改动可以使用本地临时分支，但临时分支只是开发安全网，不代表平台或版本线；完成后立即合入 `main` 并删除。
- 远程仓库长期只保留 `main`。本地临时分支默认不推送，也不以 Pull Request 作为个人开发的必经流程。
- `main` 中不允许保留明知无法构建、未经验证或带有临时实验性质的提交。
- 所有平台共用同一条提交历史和同一条版本线。

## 3. 本地临时分支

以下情况建议使用本地临时分支：

- 跨多个共享 crate 的修改；
- 文件格式、文档模型、持久化数据或兼容性行为变更；
- 大规模依赖升级；
- 可能同时影响多个平台的 UI、输入、渲染或文件操作修改；
- 需要多次实验，暂时无法通过完整验证的修改；
- 需要保留当前稳定位置以便随时修复其他问题的修改。

临时分支命名为：

```text
local/<short-topic>
```

例如：

```text
local/ios-file-picker
local/shared-touch-input
local/dependency-refresh
```

使用规则：

- 分支名使用小写英文单词和连字符，描述任务，不使用平台长期开发线命名。
- 不创建 `pc-*-wip` 等含义不清、容易长期遗留的暂存分支。
- 临时分支只保存一个主题；完成后使用 rebase 保持线性历史，再合入 `main`。
- 合入并通过验证后立即删除临时分支，不将其变成新的长期分支。
- 临时分支不得直接推送为远程平台分支；确需远程备份时必须先获得仓库所有者同意，并在用完后及时删除。

## 4. 同步、合入与历史

- 开始新工作前先同步远程 `main`。
- 推送前获取远程更新，并只对尚未推送的本地提交执行 rebase：

```sh
git fetch origin
git pull --rebase origin main
```

- 已经推送到远程的提交不得改写、重新排序或删除。
- 禁止对 `main` 执行 force-push，禁止使用强制推送覆盖远程历史。
- 以线性历史为主，不再进行 `main` 与平台开发分支之间的双向 merge。
- 本地临时分支合入时优先使用 rebase 后 fast-forward；确需保留合并节点时，必须有明确理由。
- 不使用 `git reset --hard`、强制检出或清理命令丢弃自己尚未确认的修改。
- 推送后发现 `main` 失败，应立即修复；无法快速修复时使用 `git revert` 回退对应提交，不改写历史。

## 5. 提交格式

提交信息使用 Conventional Commits：

```text
<type>(<scope>): <subject>
```

常用 type：

- `feat`：新增用户可见能力；
- `fix`：修复缺陷；
- `refactor`：不改变外部行为的重构；
- `perf`：性能优化；
- `test`：测试相关修改；
- `build`：构建、打包或版本配置；
- `ci`：持续集成配置；
- `docs`：文档；
- `style`：不改变行为的格式调整；
- `chore`：其他维护工作。

常用 scope：

- 平台：`ios`、`android`、`mobile`、`macos`；
- 共享模块：`engine`、`ui`、`compose`、`io`、`comfy`；
- 维护：`deps`、`packaging`、`release`。

示例：

```text
feat(ios): support document picker cancellation
fix(android): restore gallery thumbnail loading
fix(engine): validate generated image dimensions
refactor(mobile): share touch gesture handling
build(macos): update bundled document types
build(release): bump all platforms to 1.0.4
docs: describe the single-main workflow
```

提交要求：

- 一个提交只表达一个逻辑变化，并保持该提交可以独立构建和审查。
- subject 使用简洁的英文祈使句，不以句号结尾。
- 提交信息中的移动产品名必须写作 **PhotoComfy**，macOS 产品名必须写作 **PhotoCraft**。
- 同一项跨平台逻辑改动可以放在一个提交中，不为每个平台机械拆分提交。
- 不将无关格式化、重构、生成文件或依赖更新混入功能提交。
- 构建脚本、项目文件和生成文档必须与对应改动一起提交，避免版本不一致。
- 不把 `wip`、临时代码、调试输出、失效实验或注释掉的大段代码推送到 `main`。
- UI 行为改动应在提交说明、开发日志或交付说明中记录验证路径；必要时附前后截图。

## 6. 跨平台代码改动

- 共享行为放在 `crates/` 中，平台特殊行为放在对应的 `apps/` 目标或平台目录中。
- 使用 Rust 条件编译、平台服务接口和构建配置处理差异，不以平台分支维护不同实现。
- 修改共享 UI、输入、文件保存、引擎或渲染代码时，必须评估 iOS、Android、macOS 的影响，不能只检查提交 scope 中的平台。
- 平台特殊优化不得无依据地改变其他平台行为；条件编译范围应准确、尽量小。
- 不引入只支持单一像素位深、颜色模型或特定平台的共享 API。
- 不破坏 WebAssembly 检查及上游要求保留的其他平台构建。
- 不提交构建产物、Xcode 用户状态、Android 签名文件、语料、个人文件或本地配置。

## 7. 测试规范：最小必要验证

测试只用于验证本次改动，不用于展示工作量，也不用于重复证明上游核心功能。与当前改动没有直接关系的测试、构建和检查都属于不必要测试，明确禁止。

### 7.1 按改动类型选择验证

| 改动类型 | 必要验证 | 明确不做 |
|---|---|---|
| 文档、Git 规范、注释 | 阅读实际 diff，运行 `git diff --check` | 不运行 Rust 构建或测试 |
| 单一平台资源、Manifest、Xcode 或打包配置 | 构建该平台对应产物 | 不运行共享核心测试或其他平台构建 |
| 单一平台独立适配代码 | 构建该平台、启动应用、手工复现本次操作路径 | 不运行全 crate、全工作区或全平台测试 |
| 平台 UI 布局或交互 | 对应平台启动和手工验证；只有视觉变化无法靠操作确认时才截图 | 不默认新增截图测试、GPU 测试或快照文件 |
| 共享 crate 行为修改 | 格式检查、受影响 crate 的 Clippy、直接覆盖本次行为的既有定向测试，以及受影响平台构建 | 不运行无关 crate、全量测试或未受影响平台测试 |
| crate 依赖或层级变化 | 在上述必要验证外运行 `cargo xtask layers` | 普通代码修改不重复运行 layers |
| wasm 条件编译、feature、共享依赖或 Web 行为变化 | 运行 `cargo xtask wasm` | 普通 iOS / Android / macOS 适配不运行 wasm |
| 正式发布 | 运行本次发布平台的打包和安装/启动冒烟验证 | 不因发版自动补跑全工作区、语料、性能或模糊测试 |

Rust 代码的格式和静态检查使用：

```sh
cargo fmt --all -- --check
cargo clippy -p <affected-crate> --lib -- -D warnings
```

既有测试只运行直接覆盖本次改动的测试名称、模块或测试文件，例如：

```sh
cargo test -p <affected-crate> <test-name> --lib
cargo test -p <affected-crate> --test <test-file> <test-name>
```

不得把上述示例机械地全部执行。能够从代码和条件编译确认平台不受影响时，不构建该平台；无法确认时才扩大验证范围。

### 7.2 禁止的不必要测试

除非仓库所有者针对当前任务明确要求，否则禁止：

- 运行 `cargo test --workspace`、`cargo nextest run --workspace`、`cargo xtask test` 或 `cargo xtask ci` 等全工作区测试。
- 运行完整 `photocraft-ui-egui` 测试套件来验证一个局部平台改动。
- 运行 `cargo xtask test-corpus`、语料下载、fuzz、`panic_hunt`、ignored 长测试、性能基准或 nightly 测试。
- 为普通适配改动运行 GPU 矩阵、视觉快照矩阵、多设备矩阵或所有平台构建。
- 为上游 PhotoCraft 核心功能新增、补充或修复测试，或等待与本次改动无关的上游既有失败。
- 修复、跳过、隔离或重写并非由本次改动引入的失败测试。
- 在测试已经通过且代码没有继续变化时，换用其他命令重复证明同一行为。
- 为了“保险”“顺便”或遵循通用最佳实践而扩大测试范围。
- 为简单配置、资源、文案、版本号或构建号修改运行 Rust 单元测试。
- 为了凑测试数量、提高覆盖率或满足形式化流程而新增测试。

### 7.3 新增测试的默认规则

**默认不新增测试文件、测试模块、测试依赖、测试脚手架、基准或 CI 测试任务。**

只有同时满足以下条件时，才允许在现有测试文件中增加或更新一个定向测试：

- 验证的是本仓库当前修改的平台适配行为，而不是上游核心功能；
- 现有测试文件已经直接覆盖同一模块和行为；
- 测试确定、快速，不依赖网络、真实设备、签名、外部语料或人工截图；
- 自动化测试明显比重复手工验证更准确且维护成本更低；
- 不引入新依赖、测试框架或无关重构。

任意一条不满足，就不新增测试，使用必要的构建和手工验证即可。不得因为其他仓库或通用规范要求“每个改动都有测试”而突破此限制。

### 7.4 验证结果记录

- 只报告实际运行的命令和结果，明确区分构建成功、测试通过和手工验证通过。
- 未运行的测试不需要泛泛列成待办；只有它会影响本次交付结论时才说明。
- 遇到无关既有失败，记录证据后继续当前任务，不扩大修改范围。
- 不得把编译成功描述为功能已验证，也不得声称未实际运行的平台测试已经通过。

## 8. 推送 main 前的平台门禁

按改动影响范围执行以下构建。

iOS：

```sh
apps/photocraft-ios/ios/build.sh out PhotoComfy iphonesimulator arm64
```

Android：

```sh
apps/photocraft-android/android/build.sh out/android
```

macOS：

```sh
cargo build -p photocraft
```

macOS 发布包验证：

```sh
packaging/macos/package.sh --arch aarch64
```

门禁要求：

- 仅修改某一平台的独立适配代码时，至少构建该平台并手工验证本次改动。
- 修改共享代码时，构建所有受影响平台；无法判断是否影响时，按三端都受影响处理。
- iOS 和 Android 改动至少确认应用能够启动，本次适配点可以按操作路径复现。
- macOS 发布前必须验证实际打包流程，而不只是 `cargo check`。
- 推送前检查：

```sh
git status
git diff --check
git log --oneline origin/main..HEAD
```

- 使用普通 `git push origin main`，禁止添加强制推送参数。
- 在自动化尚未覆盖某一平台时，该平台的本地构建和手工验证就是推送前的必要门禁，不能省略后声称已验证。

## 9. 统一版本号规范

- 所有平台共用同一条 Semantic Versioning 版本线：`主版本.次版本.修订号`。
- Git tag 统一使用 `vX.Y.Z` 格式，例如：

```text
v1.0.3
v1.0.4
v1.1.0
v2.0.0
```

- 所有发布 tag 都只能使用上述格式，不使用平台前缀、产品前缀、日期、构建号或其他后缀。
- 禁止 `ios-v1.0.3`、`android-v1.0.3`、`macos-v1.0.3`、`release-1.0.3`、`v1.0.3-build4` 等命名。
- `v` 只出现在 Git tag 和展示标签中；`Cargo.toml`、iOS `MARKETING_VERSION`、Android `versionName` 等版本字段使用不带 `v` 的 `1.0.3`。
- 主版本号：不兼容变更；次版本号：向后兼容的新功能；修订号：向后兼容的缺陷修复。
- 各平台统一版本号，不因只有一个平台发布产物就创建带平台名称的特殊 tag。
- iOS `CURRENT_PROJECT_VERSION` 和 Android `versionCode` 是商店构建序号，不属于 Git tag；每次发布必须严格递增，不能复用或回退。
- 首次统一版本时不得低于任何平台已经发布的最高版本；从该版本之后按变更类型递增。
- 版本号只在一个明确的版本提交中统一修改，避免功能提交夹带发版变更。

## 10. 基于 tag 的发布流程

- 版本通过 annotated Git tag 发布，不使用长期 `release` 分支。
- 一个 tag 标记 `main` 上一个确定、不可变的发布提交。
- 标准流程：
  1. 确认本地 `main` 与远程同步，工作区干净。
  2. 创建统一的版本号提交，更新需要发布的平台版本字段及构建序号。
  3. 按第 7 节完成必要验证，并运行本次发布平台的打包和安装/启动冒烟验证，不额外运行全量测试。
  4. 推送 `main`，确认自动化结果。
  5. 在该提交上创建 annotated tag，例如：

     ```sh
     git tag -a v1.0.3 -m "Release v1.0.3"
     ```

  6. 经仓库所有者明确同意后推送 tag：

     ```sh
     git push origin v1.0.3
     ```

  7. 由 tag 或受控的手动工作流构建发布产物。
- 发布工作流未支持 tag 前，应先调整工作流，不得重新引入长期 `release` 分支作为替代。
- tag 创建后不得移动、重用或指向另一个提交；版本有误时递增版本并创建新 tag。
- 已发布版本的问题通过新的 `main` 修复提交和新的修订版本解决，不建立长期 hotfix 分支。
- 发布说明必须写明实际构建和验证过的平台；未构建或未验证的平台不得宣称已经发布或通过验证。

### 10.1 GitHub 仓库与 Android / macOS Releases

- 本仓库的 GitHub HTTPS 地址固定为：

```text
https://github.com/huairuiLove/phcraft-comfy.git
```

- `origin` 必须指向上述仓库；使用与其等价的 SSH 地址 `git@github.com:huairuiLove/phcraft-comfy.git` 也可以。不得擅自改指其他仓库或添加新 remote。
- Android 和 macOS 均通过 **GitHub Releases** 发布，不要求分别上架 Google Play 或 Mac App Store。
- 每次发布使用统一的 `vX.Y.Z` annotated tag，并在同一个 GitHub Release 中提供本次实际构建和验证的平台产物。
- GitHub Release 只承担构建、托管和下载分发，不能替代 Apple 签名与公证，也不能替代 Google Play 的商店审核。

#### Android 发布要求

- GitHub Release 上传签名后的 APK，标准产物命名为：

```text
PhotoComfy-<version>-android-arm64.apk
```

- 例如版本 `1.0.4` 对应 `PhotoComfy-1.0.4-android-arm64.apk`，Git tag 为 `v1.0.4`，APK 的 `versionName` 为 `1.0.4`。
- 每次发布必须递增 Android `versionCode`，不得继续固定为 `1`，不得复用或回退。
- 正式发布必须使用固定、可恢复并妥善备份的 Android release keystore；不得使用每个 checkout 自动生成的本地调试 keystore 签名。
- release keystore 和密码只能保存在受控的本地环境或 GitHub Secrets 中，不得提交到仓库、日志或 Release 说明。
- 发布前必须验证 APK 签名、覆盖安装、应用启动以及本次改动路径；签名变化导致无法覆盖安装时，不得把该 APK 作为普通更新发布。
- GitHub Releases 直接分发 APK；AAB 仅用于需要它的 Google Play 等商店流程，不替代 GitHub APK 产物。
- 当前仓库没有 Android GitHub Actions 构建和发布任务。实施统一 tag 发布前，必须增加 Android release 构建、签名、产物校验和 Release 上传步骤，不能只上传本地未验证的 APK。

#### macOS 发布要求

- GitHub Release 上传 Apple Silicon 与 Intel 通用 DMG，标准产物命名为：

```text
PhotoCraft-<version>-macos-universal.dmg
```

- 例如版本 `1.0.4` 对应 `PhotoCraft-1.0.4-macos-universal.dmg`，Git tag 为 `v1.0.4`。
- 正式 macOS 产物必须使用 Apple Developer ID 签名，启用 hardened runtime，完成 Apple notarization，并将公证票据 stapling 到应用和 DMG。
- 签名证书、Apple ID、app-specific password、Team ID 等凭据只能放在受控的 GitHub Secrets 或发布环境中。
- 发布前必须验证 DMG、`codesign`、公证票据、Gatekeeper 评估以及安装后的启动；ad-hoc 签名包只能用于本地检查，不得冒充正式发布包。
- 现有 macOS 发布流程可以继续生成 universal DMG，但触发方式必须迁移为 `vX.Y.Z` tag，不再依赖长期 `release` 分支。

#### 统一 GitHub Release 产物与检查

一个同时发布 Android 和 macOS 的 Release 至少包含：

```text
PhotoComfy-<version>-android-arm64.apk
PhotoCraft-<version>-macos-universal.dmg
SHA256SUMS.txt
```

- `SHA256SUMS.txt` 必须覆盖本次 Release 的全部 APK、DMG 和其他下载产物。
- 上传前核对产物名称、版本号、架构、签名和文件大小；上传后核对 GitHub Release 中的资产是否完整。
- Release 说明分别列出 Android 和 macOS 的改动、安装方式及实际验证结果，不宣称未构建或未验证的平台已经发布。
- 当前 `.github/workflows/release.yml` 仍由 `release` 分支触发。正式采用本规范前，必须改为 `v*` tag 或受控手动触发，并接入 Android 任务；文档规定不得被解释为现有自动化已经具备这些能力。

## 11. App Store Connect 固定上传流程

### 11.1 上传版本的 AIGC 合规要求

- 所有上传到 App Store Connect 的 PhotoComfy 版本，包括 TestFlight 和 App Store 审核版本，都必须在编译期完整移除 AIGC 功能。
- 上传版本必须使用 `PHOTOCOMFY_DISABLE_AIGC=YES`，使 `apps/photocraft-ios/ios/build.sh` 执行：

```sh
cargo build --profile release --target aarch64-apple-ios --offline --no-default-features --features heif -p photocraft-ios
```

- “完整移除”至少要求：
  - 不编译 `src/api_gen.rs` 和可选的 Image API 网络 transport；
  - `comfy_transport`、`comfy_image_api`、`comfy_generations_dir` 均处于禁用状态；
  - 不出现 Generate、ComfyUI、Image API、模型、endpoint、API key 等 AIGC 入口或设置；
  - 不读取、展示或使用旧的 Image API 配置，不保留可用的新增生成记录目录；
  - 不存在可触发 AIGC 请求的命令路径、远程开关或隐藏入口；
  - 不发出任何 AIGC 网络请求。
- 不得仅在审核期间隐藏按钮、通过远程配置关闭功能或依赖运行时判断来伪装合规。功能必须从上传构建中实际移除。
- 本地 Debug 或开发构建可以保留 AIGC，但任何包含默认 `aigc` feature 的构建都不得上传 App Store Connect。
- 当前 Xcode 项目中的 `PHOTOCOMFY_DISABLE_AIGC = NO` 不能作为上传配置。归档时必须显式传入 `PHOTOCOMFY_DISABLE_AIGC=YES`，并核对日志出现：

```text
AIGC transport: disabled (explicit override)
```

- 日志显示 `AIGC transport: enabled`、缺少禁用日志或无法确认 feature 时，该 archive 一律禁止上传。
- 上传前使用同一个无 AIGC 配置启动应用，手工确认生成入口和相关设置不存在，其他编辑、打开和保存功能仍可使用。这里只验证合规点和本次改动，不新增或运行无关测试。

### 11.2 每次更新的版本号要求

- 每次功能更新、修复更新或合规更新后，先递增 `MARKETING_VERSION`，至少递增修订号，再执行上传流程。
- 每次实际上传到 App Store Connect 的构建都必须递增 `CURRENT_PROJECT_VERSION`，包括同一 `MARKETING_VERSION` 下的重新上传。
- `MARKETING_VERSION` 与 Git tag 使用同一版本线，例如版本字段为 `1.0.4`，tag 为 `v1.0.4`。
- `CURRENT_PROJECT_VERSION` 使用正整数，每次加一，不复用、不回退，也不写入 Git tag。
- 版本修改必须同步所有实际读取该值的位置，包括：
  - 根 `Cargo.toml` 与必要的 `Cargo.lock`；
  - `PhotoComfy.xcodeproj` 中 Debug 和 Release 的 `MARKETING_VERSION`、`CURRENT_PROJECT_VERSION`；
  - `ios/build.sh` 生成独立 Info.plist 时使用的回退版本值。
- 上传前检查 archive 中的 `CFBundleShortVersionString` 和 `CFBundleVersion`，确认与本次版本提交一致。
- 构建在本地失败且从未上传时，可以修复后继续使用该构建号；一旦 App Store Connect 已接收该构建号，再次上传必须递增。

### 11.3 固定执行顺序

每次更新后按以下顺序直接执行，不改用默认 Xcode Release 配置，不只生成 archive 后停止：

1. 完成功能改动和第 7 节规定的最小必要验证。
2. 按第 11.2 节递增版本号和构建号，形成独立版本提交。
3. 确认 `main` 已同步、工作区干净，并确认本次上传版本不包含 AIGC。
4. 使用无 AIGC 配置完成应用启动和合规点手工检查。
5. 在仓库根目录使用独立归档目录执行 clean archive：

   ```sh
   export PHOTOCOMFY_DISABLE_AIGC=YES
   xcodebuild \
     -project apps/photocraft-ios/ios/PhotoComfy.xcodeproj \
     -scheme PhotoComfy \
     -configuration Release \
     -destination 'generic/platform=iOS' \
     -archivePath out/app-store/PhotoComfy.xcarchive \
     PHOTOCOMFY_DISABLE_AIGC=YES \
     clean archive
   ```

6. 检查归档日志、AIGC 禁用状态、签名结果和 archive 版本信息；任何一项不符都停止上传并重新构建。
7. 使用现有 `ExportOptions.plist` 导出并直接上传。该文件已经设置 `method = app-store-connect` 和 `destination = upload`：

   ```sh
   xcodebuild \
     -exportArchive \
     -archivePath out/app-store/PhotoComfy.xcarchive \
     -exportPath out/app-store/export \
     -exportOptionsPlist apps/photocraft-ios/ios/ExportOptions.plist
   ```

8. 上传后继续按既定 App Store Connect 流程提交该无 AIGC 构建供审核，不等待另行提醒，不上传包含 AIGC 的对照包。
9. 确认 App Store Connect 已接收对应的版本号和构建号，记录上传结果。`Processing` 只表示 Apple 正在处理，不得描述为审核通过。
10. 推送 `main`，并在同一版本提交上创建和推送 `vX.Y.Z` annotated tag；tag 操作不得替代或中断 App Store Connect 上传。

按照本小节完成的 App Store Connect 例行上传和提交审核属于仓库所有者已授权的固定发布流程，无需每次再次询问。任何绕过 AIGC 禁用、修改签名身份、替换发布账号、覆盖已有构建或变更导出配置的操作不在此授权内。

### 11.4 失败与重新上传

- 签名、导出或上传失败时，先报告真实错误，不得改用包含 AIGC 的构建绕过问题。
- 已上传但发现 AIGC 未完全移除、版本信息错误或合规点不符时，不得复用该构建提交审核；递增构建号并重新上传无 AIGC 版本。
- 若错误涉及实际功能或版本内容，按变更类型递增 `MARKETING_VERSION`，使用新的 `vX.Y.Z` tag。
- 不删除或覆盖 App Store Connect 中已有的构建记录，不宣称 Apple 会批准某个版本；审核结论以 App Store Connect 的实际状态为准。

## 12. 分支与 worktree 清理

- 删除任何分支前，确认其提交已经并入 `main`，或已经在仓库外完成可恢复的备份。
- `git branch --merged main` 只是检查依据之一；仍须检查未合并提交、开放 PR 和关联 worktree。
- 删除 worktree 前必须检查未提交修改和未跟踪文件，不能只根据分支是否已合并判断安全性。
- WIP 恢复分支、detached worktree 和暂存修改必须先审查、合入或备份，再清理。
- 已合并的平台分支可以从本地和远程删除，但远程删除必须遵守第 14 节的确认要求。
- 不保留“可能以后有用”的长期分支；需要留存的历史优先使用不可变 tag 或仓库外备份。

## 13. 文件、密钥与生成物

- 不提交密钥、证书、 provisioning profile、keystore、签名密码、访问令牌或个人隐私内容。
- 不提交字体文件、测试语料、个人照片和未获许可的素材。
- 不提交 `target/`、`out/`、`dist/`、APK、App、DMG、DerivedData、日志或临时截图，除非该路径明确属于需要版本控制的发布素材。
- Android 更新必须沿用正确的签名身份；签名文件只保存在受控的本地或发布环境中。
- 构建脚本生成且要求版本控制的文件必须与源改动一起更新；其他生成物不得入库。

## 14. 危险操作先问人

以下操作必须先获得仓库所有者的明确同意：

- 删除远程分支或 tag；
- force-push、改写已发布历史或移动已有 tag；
- 创建并推送正式发布 tag、发布或覆盖 Release；
- 修改签名配置、secrets、GitHub Environments 或发布权限；
- 花钱、购买服务或触发可能产生费用的操作；
- 添加新 remote；
- 批量删除 worktree、未提交修改、未跟踪文件或本地备份。

第 11 节明确授权的 App Store Connect 例行上传和提交审核除外；该流程直接执行，不逐次询问。除上述情况及既定例外外，按本规范继续完成工作，不因普通实现决策反复询问。

## 15. Agent 交付要求

- 开始工作前阅读本文件、存在的 `AGENTS.md` 以及相关目录中的更具体规范；本文件规定本仓库的单主干和三端发布流程。
- 修改应聚焦当前任务，不顺手修复无关问题，不改变无关平台行为。
- 修改文档或规范后，检查是否还有同仓库文件描述旧的长期平台分支、`release` 分支流程或包含 AIGC 的 App Store 上传流程，并在交付说明中列出需要同步的内容。
- 完成后审查实际 diff，确认提交范围、命名和文档与实现一致。
- 交付说明必须列出实际运行的验证命令和结果；未运行的验证必须明确说明。
- 不创建未经验证的“已完成”结论，不隐瞒失败，不将上游既有失败冒充为本次改动引入的问题。
- 按仓库要求更新 `log/devlog.md`，记录完成内容、验证结果和仍未解决的事项。
