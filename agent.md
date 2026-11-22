# agent.md — 本仓库 Agent 规范（仅三端适配）

> 本仓库是 PhotoCraft 的三端适配仓库：iOS / Android / macOS 三个 **PhotoComfy** 客户端。
> 本文件只约束三端适配工作。PhotoCraft 引擎与桌面端功能本身的开发与测试属于上游仓库的职责：
> **不要为本仓库添加、修复或等待 PhotoCraft 核心功能的测试**。CI 中核心功能的既有失败不阻塞三端合并。

## 1. 产品与命名

- 三端移动应用统一叫 **PhotoComfy**（机器名 `photocomfy`）：iOS 的 Xcode 工程是 `PhotoComfy.xcodeproj`、bundle id `photocomfy`；Android 的 applicationId 以 `photocomfy` 开头；macOS 桌面端沿用 PhotoCraft 名称。
- 历史路径 `apps/photocraft-ios/`、`apps/photocraft-android/` 与 crate 名保持现状，不要重命名。
- 共享 Rust 核心（`crates/`）随上游演进；适配改动优先落在 `apps/` 与平台胶水层，尽量小。

## 2. 分支与合并

- `main` 只通过 Pull Request 进入；三端日常开发分别在 `ios-dev`、`android-dev`、`macos-dev`，阶段性合入 `main`。
- 个人暂存分支（`pc-*-wip` 等）不得直接合入 `main`。
- 长期分支合入用 merge commit，信息与现有历史一致：`Merge branch 'ios-dev'`。
- 不对共享分支 force-push 或改写历史。

## 3. 提交

- Conventional Commits：`feat` / `fix` / `build` / `chore` / `docs` / `style` 等；scope 用 `ios` / `android` / `mobile` / `macos`。
- 提交信息里的移动端产品名必须是 PhotoComfy。
- 一个提交一个主题；涉及移动端 UI 行为的改动在描述里附截图或操作路径。

## 4. 验证（只验证三端适配本身）

- 改动后至少确认：对应平台可构建（iOS：`ios/build.sh` 与 Xcode 工程；Android：APK 构建）、应用可启动、本次改动的适配点手工可复现。
- 不为 PhotoCraft 引擎/桌面端功能新增或修复测试；如需对齐上游核心改动，让适配层跟上即可。
- 不提交密钥、证书、签名材料、字体文件、个人隐私内容；语料（corpus）不入库。

## 5. 危险操作先问人

删除远程分支或 tag、改写历史、发版打 tag、动 secrets、花钱、添加新 remote——先获得仓库所有者的明确同意。其余情况不要停下来问，继续干活。
