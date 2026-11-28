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

### 1.1 macOS 端 PhotoCraft 改动禁令

- 在没有获得仓库所有者针对当前任务的明确允许前，严禁对 macOS 端 PhotoCraft 进行任何改动。
- “任何改动”包括但不限于：`apps/photocraft/` 桌面端代码、UI、资源与配置，macOS 构建与打包配置（含 `packaging/macos/`），macOS 版本号与发布配置，macOS 专属文档，以及任何以 macOS 端为目的地的构建、打包、签名、公证和发布操作。
- 本禁令不限制三端共享 `crates/` 的共同维护；但共享代码改动不得夹带 macOS 端专属行为、配置或开关，变相修改 macOS 端。
- 明确允许必须针对当前任务专门作出，不得从过往授权、其他平台授权或第 11 节 iOS 发布授权中推断。
- 违反本禁令的改动一律不得提交或推送；发现误改时立即恢复原状并在交付说明中报告。

### 1.2 iOS 端使用 Xcode 工具链构建、归档和上传

- iOS / iPadOS 使用 `apps/photocraft-ios/ios/PhotoComfy.xcodeproj` 中的 scheme、签名团队和配置。允许通过 **Xcode 图形界面**或命令行 `xcodebuild` 开发、构建、签名、归档和上传；允许使用 `xcrun` 配合 Xcode 工具链完成 App Store Connect 上传。
- 仓库所有者已明确授权按第 11 节通过命令行上传 App Store Connect，无需逐次确认。使用本机已配置的 Xcode 签名、App Store Connect API Key 或钥匙串凭据；不得在聊天、日志或仓库中索取、输出或保存密码、私钥及完整凭据内容。
- 需要运行验证时，可以在 Xcode 中选择 iOS / iPadOS Device 并执行 **Run**，通过 Xcode 自带的 DeviceHub 查看模拟设备；也可以在 Xcode 中选择已连接的真机运行。
- 共享 Rust 代码仍可按第 7 节运行主机上的格式检查、Clippy 和定向测试；这些检查不属于 iOS 构建，也不能替代 Xcode 构建及 Device 上的实际验证。
- iOS 交付必须明确记录实际构建、归档、上传结果和已执行的手工验证；未实际执行时不得声称 iOS 已验证或上传成功。上传成功仅代表 App Store Connect 已接收构建，不代表通过 App Review。

### 1.3 iOS 与 Android 的绘制输入控制必须严格分开

- iOS 版本和 Android 版本必须严格分开画笔等一切绘制的控制逻辑。谁可以绘画、填充和选择，由各平台外壳自己决定：这套决定只存在于 iOS 外壳（`apps/photocraft-ios/src/drawing.rs`）；共享代码只保留与平台无关的策略闸门（`crates/ui-egui/src/input/draw_policy.rs`），不得再出现按平台条件编译的绘制判断，也不得让一端的规则通过共享代码作用到另一端。
- iOS 端必须严格禁止用手指进行画笔、喷涂、选中、擦除等一切绘制类操作。在 iPad 上只有 Apple Pencil 可以驱动绘制类工具：画笔家族与各种橡皮擦、修复与仿制、填充与渐变、全部选区工具、钢笔与形状工具、文字蒙版，以及液化、内容识别填充、选择并遮住、消失点等工作区里的绘画手势。手指只用于界面和画布导航（双指平移缩放、长按菜单）；用它做绘制类操作时画布不响应，状态栏提示“绘画、填充和选区需要使用 Apple Pencil”。鼠标、触控板和自动化不受限。
- iPhone 相当于严格禁止任何画画、填充和选中功能，手指和 Pencil 都不行：工具栏不显示绘制类工具，工具命令不能选中它们，「选择」菜单以及「编辑 › 填充 / 描边 / 内容识别填充」、「图层 › 新建填充图层」、液化等命令置灰，画布拒绝一切此类手势，状态栏提示“此设备不支持绘画、填充和选区”。
- 判断设备是 iPad 还是 iPhone 只能用系统报告的设备类型（UIKit 的 `userInterfaceIdiom`），不得用屏幕尺寸、分辨率或方向推断，以免部分 iPad 被误判成 iPhone 而失效。
- Pencil 绘画的可靠性同样属于 iOS 的绘制控制：手指先落屏时 Pencil 落下必须接管指针并立刻开始笔画；笔画进行中手指（手掌）落下不得取消或移动笔画；Pencil 停留不得触发长按菜单；被系统取消的画布按压必须释放，不能让指针卡在按下状态。
- Android 端不做上述任何特殊处理：手指照常绘画、填充和选择，Android 外壳不接入这套策略，其代码也不随 iOS 的绘制控制改动。
- 修改任一端的绘制控制后，必须验证本端在模拟器或真机上的行为（iOS 用 `PHOTOCRAFT_DRAW_PROBE=1` 回放手指与 Pencil 触摸，并运行 `cargo test -p photocraft-ios`），同时用共享层定向测试（`cargo test -p photocraft-ui-egui --lib -- draw_policy input::finger`）确认默认策略下任何输入照常绘画，即另一端行为未变。

### 1.4 中文字体必须在字体菜单里可直接辨认

- 仓库所有者的要求是能在字体选项里找到并使用中文字体，不只是字体被编进包里。中文界面下，Type 工具及其他位置的字体菜单必须：在最近使用的字体之后列出“中文字体”分组；分组里的字体以界面语言的名字显示（字体自带的中文名，或字体包清单 `name` 行给出的名字）；覆盖中文的字体在样张栏显示中文样张。日文、韩文界面同理；英文等非 CJK 界面的菜单保持原样。
- 改动字体菜单、字体包或字体嵌入方式后，必须在模拟器或真机上实际点开字体菜单确认分组、名字和样张，并选一个中文字体输入中文；不能只凭渲染样张或单元测试宣称完成。没有辅助功能权限时，用一个独立的 XCUITest（`XCUIApplication(bundleIdentifier:)` 加坐标点击）驱动模拟器。
- 主机验证（需嵌入字体包）：`CRAFT_FONTS_DIR=$PWD/craft-fonts PHOTOCOMFY_FONTS_DIR=$PWD/photocomfy-fonts cargo test -p photocraft-ui-egui --lib -- font_picker_tests:: font_preview`，以及 `cargo test -p photocraft-text --lib -- native_family_names a_native_name_follows`。详见 `docs/photocomfy-fonts.md`。

## 2. Git 核心原则

- `main` 是唯一的长期分支，也是远程仓库的默认分支。
- 不建立长期存在的 `develop`、`release`、`ios-dev`、`android-dev`、`macos-dev`、平台分支或 hotfix 分支。
- 小型、低风险且已完成验证的改动可以直接提交到本地 `main`，验证后推送。
- 高风险改动可以使用本地临时分支，但临时分支只是开发安全网，不代表平台或版本线；完成后立即合入 `main` 并删除。
- 远程仓库长期只保留 `main`。本地临时分支默认不推送，也不以 Pull Request 作为个人开发的必经流程。
- `main` 中不允许保留明知无法构建、未经验证或带有临时实验性质的提交。
- 所有平台共用同一条提交历史和同一条版本线。

### 2.1 单人多设备与多 Agent 协作

本仓库虽然只有一个维护者，但不同设备上的 Agent 可能并行工作。协作以 GitHub `origin/main` 为唯一共享事实来源；任何设备、Agent、worktree 或本地目录都不是其他设备可以默认访问的“最新版本”。

#### 2.1.1 任务分配与文件边界

- 每个 Agent 只处理一个边界明确的任务，并在开始时确认自己的平台、模块和允许修改的路径。
- 并行任务必须尽量修改不同模块或不同文件；同一文件不得由两个 Agent 同时修改。
- 下列共享文件同一时刻只允许一个任务修改：
  - `agent.md`、根 `Cargo.toml`、`Cargo.lock`；
  - 共享 crate 的公共入口、依赖配置和构建脚本；
  - `PhotoComfy.xcodeproj`、Android Manifest、各平台发布工作流；
  - 版本号、构建号、发布配置和自动生成的汇总文档。
- 第二个任务如果必须修改正在被占用的共享文件，应先选择其他独立工作；无法避开时等待前一个任务推送并完成交接，不在旧副本上继续堆积改动。
- 不因为使用不同设备就默认按平台长期占有目录。任务完成后，文件边界随之释放。
- 一个 worktree 同一时刻只能由一个 Agent 使用；不得让多个 Agent 在同一工作目录中交错编辑、暂存、提交或清理。
- 不使用 iCloud、Dropbox、网盘同步目录或共享挂载目录作为 Git worktree，避免文件锁和同步冲突。
- 设备名、Agent 名和任务名只用于本地交接说明，不写入提交作者、Co-authored-by 或产品代码。

#### 2.1.2 每个设备开始工作前

先执行：

```sh
git status --short --branch
git remote --verbose
git fetch origin
git rev-list --left-right --count origin/main...main
```

然后遵守：

- 确认 `origin` 指向第 10.1 节规定的仓库，当前分支为 `main` 或本任务的本地临时分支。
- 只有工作区干净时，才执行 `git pull --rebase origin main`。
- 工作区存在修改或未知文件时，先判断是否是其他 Agent 的工作；不得直接 pull、stash、reset、clean、提交或删除。
- 发现其他 Agent 的未提交工作时，不接管、不覆盖；改在独立 worktree 中开始不冲突的任务，或等待明确交接。
- 本地有未推送提交而远程已经前进时，只 rebase 自己的未推送提交；不得用强制推送让远程回退。
- 开始实现前阅读最新 `agent.md` 和实际代码，不能依赖其他设备会话中的旧描述。

#### 2.1.3 开发期间与推送竞争

- 完成一个可独立验证的原子提交后及时推送，不在单台设备上积累多日或多个主题的未推送提交。
- 不把未完成工作、半成品或仅用于让另一台设备看见的 WIP 提交推送到 `main`。
- 推送前再次执行 `git fetch origin`，检查远程是否被其他设备更新；有更新时先 rebase 本地未推送提交。
- rebase 引入与本次任务相关的代码变化后，重新执行必要的最小验证；不能沿用 rebase 之前的验证结论。
- 推送被拒绝时视为正常的并发同步信号：重新 fetch、rebase、检查冲突并普通推送，禁止 force-push。
- 解决冲突时逐段理解双方意图，不得对整个文件机械使用 `ours` 或 `theirs`，不得删除其他设备刚合入的功能。
- 遇到与当前任务无关的冲突或他人未完成工作时，不扩大修改范围；保留现场并等待明确交接。
- 自动生成文件由同一任务一次性生成并提交，其他 Agent 不在不同设备上重复生成后覆盖。
- 不把解决并发冲突、格式化全仓库或修复无关失败作为当前任务的一部分。

#### 2.1.4 未完成任务跨设备交接

- 只有已经推送到 GitHub 的提交、tag 和 Release 才能被其他设备直接获取；`log/`、本地临时分支、stash、未提交修改和本地截图不会自动同步。
- 尽量在一个设备上完成任务的最小验证、原子提交和推送，再由另一个设备继续；优先避免交接半成品。
- 必须交接未完成任务时，交接说明至少包括：
  - 当前基线提交和本地提交哈希；
  - 修改过的文件及每个文件的目的；
  - 未提交、未跟踪和有意忽略的文件；
  - 已运行的验证及结果；
  - 剩余步骤、已知失败和不能修改的边界。
- 未推送工作需要转移时，优先使用审查过的 patch 或 Git bundle，通过仓库所有者认可的安全渠道传输；传输前检查其中不含密钥和个人信息。
- 确需通过 GitHub 转移未完成工作时，必须获得仓库所有者明确同意，使用短期 `sync/<device>-<topic>` 分支，接收方取得后及时删除；该分支不是新的长期开发线，也不得直接合入 `main`。
- 接收设备必须先审查交接内容再继续，不得盲目应用 patch、覆盖本地修改或提交来源不明的文件。

#### 2.1.5 发布与版本号互斥

- 同一时刻只能指定一个设备和一个 Agent 作为发布协调者，负责版本提交、tag、GitHub Release 和 App Store Connect 状态。
- 不同设备不得同时递增版本号、创建同一个 tag、上传相同构建或分别发布互相覆盖的 Release。
- 发布前必须 fetch 最新 `main` 和全部 tag，并检查 App Store Connect 已接收的最新构建号；本地历史不是版本是否已使用的唯一依据。
- iOS、Android、macOS 即使由不同设备构建，也由同一个发布协调者统一版本和 Release 归属。
- 发布提交和 tag 推送完成后，其他设备先同步再继续后续版本修改，避免重复版本或构建号。

#### 2.1.6 任务完成与设备交接报告

每个 Agent 结束任务时必须明确报告：

- 最终提交哈希，以及是否已经推送到 `origin/main`；
- 实际修改的文件和未纳入提交的工作区状态；
- 实际运行的最小必要验证及结果；
- 仍未完成的事项和下一设备需要注意的冲突边界。

未推送时不得声称其他设备已经获得改动；推送后也不得声称其他设备已经自动同步，其他设备仍需按第 2.1.2 节主动更新。

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
build(release): bump all platforms to 1.5
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

iOS：在 Xcode 中打开 `apps/photocraft-ios/ios/PhotoComfy.xcodeproj`，选择 PhotoComfy scheme 和 iOS / iPadOS Device，执行 **Run**；严格按第 1.2 节操作，不使用命令行或其他软件替代。

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

- 所有平台共用同一条版本线，统一使用两段式 `主版本.次版本`（`1.1`、`1.2`、`1.3` 这种写法），与 App Store Connect 中的版本号写法一致。
- Git tag 统一使用 `vX.Y` 格式，例如：

```text
v1.1
v1.2
v1.3
v2.0
```

- 所有发布 tag 都只能使用上述格式，不使用平台前缀、产品前缀、日期、构建号、修订号或其他后缀。
- 禁止 `ios-v1.2`、`android-v1.2`、`macos-v1.2`、`release-1.2`、`v1.2-build9`、`v1.2.1` 等命名。
- `v` 只出现在 Git tag 和展示标签中；iOS `MARKETING_VERSION`、Android `versionName` 等版本字段使用不带 `v` 的 `1.2`。例外是根 `Cargo.toml`：Cargo 强制包版本写成三段，因此写作 `1.2.0`（末位固定为 0，`cargo xtask version set 1.2` 会自动补上）；读取它的脚本把结尾的 `.0` 去掉再当作发布版本。
- 主版本号：不兼容变更；次版本号：其余每一次发布（新功能或缺陷修复）至少递增一次。
- 历史 tag `v1.1.0`、`v1.1.1` 保持不动；自 `v1.2` 起一律两段式。新版本不得低于任何平台已经发布的最高版本（`1.1.1` 之后是 `1.2`）。
- 各平台统一版本号，不因只有一个平台发布产物就创建带平台名称的特殊 tag。
- iOS `CURRENT_PROJECT_VERSION` 和 Android `versionCode` 是商店构建序号，不属于 Git tag；每次发布必须严格递增，不能复用或回退。
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
     git tag -a v1.2 -m "Release v1.2"
     ```

  6. 经仓库所有者明确同意后推送 tag：

     ```sh
     git push origin v1.2
     ```

  7. 由 tag 或受控的手动工作流构建发布产物。
- 发布工作流未支持 tag 前，应先调整工作流，不得重新引入长期 `release` 分支作为替代。
- tag 创建后不得移动、重用或指向另一个提交；版本有误时递增版本并创建新 tag。
- 已发布版本的问题通过新的 `main` 修复提交和新的版本（递增次版本号）解决，不建立长期 hotfix 分支。
- 发布说明必须写明实际构建和验证过的平台；未构建或未验证的平台不得宣称已经发布或通过验证。

### 10.1 GitHub 仓库与 Android / macOS Releases

- 本仓库的 GitHub HTTPS 地址固定为：

```text
https://github.com/LuyiCat/phcraft-comfy.git
```

- `origin` 必须指向上述仓库；使用与其等价的 SSH 地址 `git@github.com:LuyiCat/phcraft-comfy.git` 也可以。不得擅自改指其他仓库或添加新 remote。（GitHub 账号已于 2026-11 从 `huairuiLove` 改名为 `LuyiCat`，仓库不变，旧地址仅作跳转。）
- Android 和 macOS 均通过 **GitHub Releases** 发布，不要求分别上架 Google Play 或 Mac App Store。
- GitHub Release 只上传 Android 和 macOS 产物，不上传任何 iOS 产物；iOS 不作为 GitHub Release 发布渠道，一律按第 11 节通过 App Store Connect 分发，不得在 Release 资产中提供 IPA、xcarchive 或其他 iOS 包。
- 每次发布使用统一的 `vX.Y` annotated tag，并在同一个 GitHub Release 中提供本次实际构建和验证的平台产物。
- GitHub Release 只承担构建、托管和下载分发，不能替代 Apple 签名与公证，也不能替代 Google Play 的商店审核。

#### Android 发布要求

- GitHub Release 上传签名后的 APK，标准产物命名为：

```text
PhotoComfy-<version>-android-arm64.apk
```

- 例如版本 `1.2` 对应 `PhotoComfy-1.2-android-arm64.apk`，Git tag 为 `v1.2`，APK 的 `versionName` 为 `1.2`。
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

- 例如版本 `1.2` 对应 `PhotoCraft-1.2-macos-universal.dmg`，Git tag 为 `v1.2`。
- 正式 macOS 产物必须使用 Apple Developer ID 签名，启用 hardened runtime，完成 Apple notarization，并将公证票据 stapling 到应用和 DMG。
- 签名证书、Apple ID、app-specific password、Team ID 等凭据只能放在受控的 GitHub Secrets 或发布环境中。
- 发布前必须验证 DMG、`codesign`、公证票据、Gatekeeper 评估以及安装后的启动；ad-hoc 签名包只能用于本地检查，不得冒充正式发布包。
- 现有 macOS 发布流程可以继续生成 universal DMG，但触发方式必须迁移为 `vX.Y` tag，不再依赖长期 `release` 分支。

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
  - 不出现 Generate、ComfyUI、Image API、模型、endpoint、API key 等 AIGC 入口或设置，包括底部抽屉的“生成”按钮、“窗口 › ComfyUI Generation”、“文件”菜单里的 ComfyUI 项，以及搜索面板和快捷键编辑器里的同名命令；上传构建由 iOS 外壳调用 `PhotocraftApp::set_generation_removed()`（`crates/ui-egui/src/generation.rs`）移除；
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
- 上传前使用同一个无 AIGC 配置启动应用并打开文档，手工确认生成入口和相关设置不存在（底部抽屉没有“生成”按钮，“文件”和“窗口”菜单没有 ComfyUI 项），其他编辑、打开和保存功能仍可使用；`cargo test -p photocraft-ui-egui --lib -- generation_tests` 必须通过。这里只验证合规点和本次改动，不新增或运行无关测试。

### 11.2 每次更新的版本号要求

- 每次功能更新、修复更新或合规更新后，先递增 `MARKETING_VERSION`（递增次版本号，例如 `1.1.1` → `1.2`、`1.2` → `1.3`），再执行上传流程。
- 每次实际上传到 App Store Connect 的构建都必须递增 `CURRENT_PROJECT_VERSION`，包括同一 `MARKETING_VERSION` 下的重新上传。
- `MARKETING_VERSION` 与 Git tag 使用同一版本线，例如版本字段为 `1.2`，tag 为 `v1.2`，根 `Cargo.toml` 为 `1.2.0`。
- `CURRENT_PROJECT_VERSION` 使用正整数，每次加一，不复用、不回退，也不写入 Git tag。
- 版本修改必须同步所有实际读取该值的位置，包括：
  - 根 `Cargo.toml`（Cargo 要求的三段写法 `1.2.0`）与必要的 `Cargo.lock`；
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
5. 在 Xcode 的项目或 scheme 设置中明确设置 `PHOTOCOMFY_DISABLE_AIGC=YES`，选择 **Any iOS Device** 执行 **Clean Build Folder** 和 **Product ▸ Archive**；或使用 `xcodebuild -configuration Release -destination 'generic/platform=iOS' archive`，并显式传入 `PHOTOCOMFY_DISABLE_AIGC=YES`。
6. 在 Xcode 构建日志或 `xcodebuild` 日志、归档内容和签名结果中检查 AIGC 禁用状态、签名及 archive 版本信息；任何一项不符都停止上传并重新构建。
7. 在 Xcode Organizer 中选择该 archive，通过 **Distribute App ▸ App Store Connect ▸ Upload** 上传；或使用 `xcodebuild -exportArchive` 配合 `ExportOptions.plist`（`destination=upload`）通过命令行上传。不得上传包含 AIGC 的构建。

8. 上传完成即进入仓库所有者的内测（TestFlight）：仓库所有者由此直接内测，是否以及何时提交审核由仓库所有者内测后决定，不自动提交审核，也不上传包含 AIGC 的对照包。
9. 确认 App Store Connect 已接收对应的版本号和构建号，记录上传结果。`Processing` 只表示 Apple 正在处理，不得描述为审核通过。
10. 推送 `main`。只推送 App Store Connect 供内测时到此为止，不创建 tag 或 GitHub Release；需要正式发布（tag、GitHub Release、提交审核）时由仓库所有者另行提出，tag 操作不得替代或中断 App Store Connect 上传。

按照本小节完成的 App Store Connect 例行上传属于仓库所有者已授权的固定流程，无需每次再次询问；提交审核、创建 tag 和 GitHub Release 不在此授权内，由仓库所有者内测后决定。任何绕过 AIGC 禁用、修改签名身份、替换发布账号、覆盖已有构建或变更导出配置的操作不在此授权内。

### 11.4 失败与重新上传

- 签名、导出或上传失败时，先报告真实错误，不得改用包含 AIGC 的构建绕过问题。
- 已上传但发现 AIGC 未完全移除、版本信息错误或合规点不符时，不得复用该构建提交审核；递增构建号并重新上传无 AIGC 版本。
- 若错误涉及实际功能或版本内容，按第 9 节递增 `MARKETING_VERSION`，使用新的版本号上传。
- 不删除或覆盖 App Store Connect 中已有的构建记录，不宣称 Apple 会批准某个版本；审核结论以 App Store Connect 的实际状态为准。

### 11.5 更新完成即上传 App Store Connect 供内测

- 仓库所有者的固定要求：任何更新（功能、修复、合规）完成并通过第 7 节的最小必要验证后，直接递增版本号并按第 11.3 节归档上传 App Store Connect，不等待另行指示，也不以“尚未真机验证”为由只停留在 `main`。仓库所有者通过 TestFlight 直接内测，真机验证由内测完成。
- 版本号依次增加：`MARKETING_VERSION` 按两段式逐次递增（`1.2` → `1.3` → `1.4` …），每次上传 `CURRENT_PROJECT_VERSION` 加一；同一版本号下的重新上传也必须递增构建号。
- 一次更新对应一次上传：版本提交之后再改了代码，就按下一个版本号重新上传，不覆盖、不复用已上传的构建号。
- 上传成功即完成本次交付。交付说明仍须写明实际上传的版本号、构建号，以及“已上传不代表通过审核”。

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

第 11 节明确授权的 App Store Connect 例行上传除外；该流程直接执行，不逐次询问。提交审核、创建正式发布 tag 和发布 Release 不属于该例外，仍按本节先问人。除上述情况及既定例外外，按本规范继续完成工作，不因普通实现决策反复询问。

## 15. Agent 交付要求

- 开始工作前阅读本文件、存在的 `AGENTS.md` 以及相关目录中的更具体规范；本文件规定本仓库的单主干和三端发布流程。
- 修改应聚焦当前任务，不顺手修复无关问题，不改变无关平台行为。
- 修改文档或规范后，检查是否还有同仓库文件描述旧的长期平台分支、`release` 分支流程或包含 AIGC 的 App Store 上传流程，并在交付说明中列出需要同步的内容。
- 完成后审查实际 diff，确认提交范围、命名和文档与实现一致。
- 交付说明必须列出实际运行的验证命令和结果；未运行的验证必须明确说明。
- 不创建未经验证的“已完成”结论，不隐瞒失败，不将上游既有失败冒充为本次改动引入的问题。
- 按仓库要求更新 `log/devlog.md`，记录完成内容、验证结果和仍未解决的事项。
