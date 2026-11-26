<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/ai.storyteller.photocraft.png" width="128" height="128" alt="PhotoComfy icon">
</p>

<h1 align="center">PhotoComfy · phcraft-comfy</h1>

<p align="center">
  <b>PhotoCraft 的跨平台适配仓库：把纯 Rust 的开源图像编辑器带到 iOS、Android 和 macOS。</b><br>
  移动端的统一应用名为 <b>PhotoComfy</b>，桌面端沿用 <b>PhotoCraft</b>。
</p>

<p align="center">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-3a3a3a?style=flat-square">
  <img alt="Platforms" src="https://img.shields.io/badge/iOS%20%C2%B7%20Android%20%C2%B7%20macOS-2f7bf5?style=flat-square">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
</p>

> [!NOTE]
> 本仓库基于上游 [PhotoCraft](https://github.com/storytold/photocraft)（ArtCraft 团队的开源 Photoshop 洁净室重实现，
> MIT / Apache-2.0 双协议）。共享的 Rust 核心（`crates/`、`docs/`、`book/`）随上游演进，我们尽量不做改动；
> **本仓库的工作是把同一套代码适配到手机和平板**：应用外壳、触摸手势、竖屏界面、平台构建与签名、发布流程。
> 想了解核心引擎的功能（图层、蒙版、调整层、PSD 往返、笔刷引擎等），请看
> [上游仓库](https://github.com/storytold/photocraft)的文档。

## 平台与产物

| 平台 | 应用名 | 分发方式 | 产物 |
|---|---|---|---|
| Android | PhotoComfy | GitHub Releases | `PhotoComfy-<version>-android-arm64.apk`（arm64-v8a，最低 Android 8.0） |
| iOS / iPadOS | PhotoComfy | App Store Connect | 由仓库维护者通过 Xcode 构建上传，不在 GitHub 分发 |
| macOS | PhotoCraft | GitHub Releases | `PhotoCraft-<version>-macos-universal.dmg` |

版本线三端统一（两段式 `1.2`），发布用 `vX.Y` annotated tag；仓库工作流与发布规范见
[`agent.md`](agent.md)。

## 这个仓库做了什么

- **移动应用外壳**（`apps/photocraft-android/`、`apps/photocraft-ios/`）：
  无 Gradle/Java 的 Android 构建（`build.sh` 直接交叉编译 `libphotocraft_android.so` 并用 aapt2/apksigner 组装签名 APK）；
  iOS 工程为 Xcode 原生工程。
- **触摸交互**：双指平移与捏合缩放、双指左滑撤销 / 右滑重做（「首选项 › 增强控件 › 触控手势」开关控制）、
  两指同时按住不误判为长按、压力触点区分手指与手写笔。
- **竖屏触控界面（Touch Interface）**：窄屏下工具条移到画布上方、面板抽屉移到画布下方；
  触屏快捷栏（撤销/重做/复制/剪切/粘贴）不再置顶，浮动窗口可以正常遮盖它；
  界面缩放按屏幕物理尺寸计算，不会自己来回抖动。
- **品牌与合规**：移动端品牌为 PhotoComfy；所有上传 App Store Connect 的构建在编译期移除 AIGC 功能
  （`PHOTOCOMFY_DISABLE_AIGC=YES`）。
- **中文字体**：iOS / Android 读不到系统字体，移动端构建除 craft-fonts 外还嵌入 PhotoComfy 中文字体包
  （思源黑体、思源宋体、霞鹜文楷、得意黑、朱雀仿宋等 19 个家族）；字体文件不入库，由
  `scripts/photocomfy-fonts.sh` 按锁文件下载并校验，详见 [`docs/photocomfy-fonts.md`](docs/photocomfy-fonts.md)。
- **发布工程**：GitHub Actions tag 驱动的 Release 工作流、Android 正式签名 keystore（不进仓库，存于受控位置与 GitHub Secrets）。

## 构建

Android（从仓库根目录，产出签名 APK 于 `out/android/PhotoComfy.apk`）：

```sh
apps/photocraft-android/android/build.sh out/android
```

iOS：用 Xcode 打开 `apps/photocraft-ios/ios/PhotoComfy.xcodeproj`，选择 PhotoComfy scheme 与目标设备后直接 Run / Archive；
也可使用 `xcodebuild` 构建、归档，并按 `agent.md` 的发布流程通过命令行上传 App Store Connect。

macOS 桌面端：

```sh
cargo build -p photocraft
```

详细构建输入（Android SDK/NDK、craft-fonts 字体等）见 [`docs/development.md`](docs/development.md)。

## 测试与验证

本仓库遵循「最小必要验证」原则：只运行与改动直接相关的定向测试，不跑全工作区套件。
适配代码的测试与验证方式（主机单元测试、kittest 界面回放、Android 模拟器清单、iOS 设备验证、发版冒烟检查）
见 **[docs/photocomfy-testing.md](docs/photocomfy-testing.md)**；各平台适配点的来龙去脉见
**[docs/photocomfy-mobile-adaptation.md](docs/photocomfy-mobile-adaptation.md)**。

## 目录速览

- `apps/` — 平台外壳：iOS、Android、桌面（`photocraft`）、web 等构建目标
- `crates/` — 共享 Rust 核心（随上游演进）
- `docs/` — 上游核心文档 + 本仓库的适配与测试文档
- `agent.md` — 本仓库的 Git 工作流、提交格式、跨平台验证与发布规范（Agent 与维护者必读）
- `.github/workflows/release.yml` — tag 驱动的 Release 构建（Android APK 签名与校验在 `android` job）

## 许可证

与上游一致，采用 **[MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE)** 双协议（任选其一），
可自由使用、修改、分发，包括商用。版权与必要声明见 [NOTICE](NOTICE)；随附字体、图标等素材的
各自许可证见 [ATTRIBUTION.md](ATTRIBUTION.md)。

ArtCraft 的名称与标识是 ArtCraft 团队的商标，不包含在上述许可证内（见
[`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt)）。

<sub>上游 PhotoCraft 由 ArtCraft 团队与社区开发；本仓库为个人维护的跨平台适配分支，与 ArtCraft 团队无隶属关系。
Adobe、Photoshop 是 Adobe Inc. 的商标，仅用于描述兼容的工作流。</sub>
