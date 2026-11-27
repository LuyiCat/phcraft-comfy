# PhotoComfy 移动端适配说明

本文件记录 `phcraft-comfy` 仓库相对上游 [PhotoCraft](https://github.com/storytold/photocraft)
的适配点：改了什么、为什么、分布在哪些文件。共享 Rust 核心（`crates/`）尽量保持与上游一致，
平台差异由 `apps/` 下的外壳、条件编译和构建配置表达。

## 产品边界

- 移动端（iOS / Android）统一叫 **PhotoComfy**，机器名 `photocomfy`；macOS 桌面端沿用 **PhotoCraft**。
- 保留历史路径 `apps/photocraft-ios/`、`apps/photocraft-android/` 与现有 crate 名，不因产品命名重命名。
- 平台功能优先复用共享引擎与 UI；不复制整套平台代码、不建平台私有实现、不用长期分支规避共享代码修改。

## 平台外壳

### Android（`apps/photocraft-android/`）

- `android/build.sh`：不用 Gradle/Java。交叉编译 `libphotocraft_android.so`
  （`cargo ndk`，`aarch64-linux-android`，min SDK 26），用 aapt2 直接链接
  `AndroidManifest.xml`（`android.app.NativeActivity`）+ 资源，zipalign 后用
  apksigner 签名。`PROFILE=debug` 构建调试版。
- 默认签名 key 是首次运行自动生成的本地开发 key（`target/android/debug.keystore`）；
  **正式发布**必须设置 `PHOTOCOMFY_ANDROID_KEYSTORE`（及 `_PASS` / `_KEY_PASS` 环境变量）
  指向固定 release keystore。GitHub Actions 的 `android` job 从 secrets 取同一 keystore。
- 嵌入字体：Android 的 NativeActivity 没有 fontconfig，构建脚本自动拾取项目根目录的
  `craft-fonts/` 检出（可用 `CRAFT_FONTS_DIR` 覆盖），否则 CJK 界面文字显示为豆腐块。

### iOS（`apps/photocraft-ios/`）

- Xcode 原生工程 `ios/PhotoComfy.xcodeproj`，Rust 侧由 build script 编译
  `aarch64-apple-ios` / simulator target。
- iOS 的开发、构建、归档和上传使用 Xcode 工具链，可通过 Xcode 图形界面操作，
  也可按 `agent.md` §1.2、§11 使用 `xcodebuild` / `xcrun` 命令行构建、归档并上传 App Store Connect。
- **AIGC 合规**：所有上传 App Store Connect 的构建必须在编译期移除 AIGC
  （`PHOTOCOMFY_DISABLE_AIGC=YES`，构建日志出现 `AIGC transport: disabled (explicit override)`）。
  本地 Debug 构建可以保留，但不得上传。

## 触摸交互（`crates/ui-egui/src/touch.rs` 及手势代码）

- 开关：「首选项 › 增强控件 › 触控手势」（`interface.touch_ui`），同时控制双指平移/捏合与滑动手势。
- 双指平移与捏合缩放；双指**快速**左滑撤销、右滑重做。判定为滑动后视图回到两指落下位置，
  抬指前不再平移/缩放（`Gesture::judge` 的判定一旦给出不再改判）。
- 两指同时按住不触发长按菜单（`PenContacts` 在两个触点期间屏蔽 egui 的 press-and-hold）。
- Android 上每个触点可能都带压力值，需要区分手指与手写笔，否则双指手势失效。

## 竖屏触控界面（Touch Interface）

- `touch::portrait()` 判定竖屏 chrome，**只在 iOS/Android 构建启用**
  （`cfg!(any(target_os = "ios", target_os = "android", test))`）；桌面和 Web 窗口无论多窄
  都保持桌面布局。`PHOTOCRAFT_FORCE_PORTRAIT` 环境变量可在任意主机上强制启用（测试辅助）。
- 布局：工具条是画布**上方**的 top panel（在 options bar 之下），面板抽屉（分组按钮 +
  选中分组的标签页和正文）在画布**下方**。浮动窗口、菜单、对话框按正常层级遮盖触屏快捷栏
  （撤销/重做/复制/剪切/粘贴），被遮盖处的触摸归上层窗口。
- 界面缩放：`sync_display_scale` 按屏幕物理尺寸（跨缩放代次不变的量）计算触摸缩放系数，
  每帧最多重算一次；避免把 `content_rect × pixels_per_point`（同一 pass 内相差一个缩放代次）
  反馈进系数造成缩放来回抖动或停在错误值。

## 绘制输入控制（iOS 与 Android 严格分开）

规范见 `agent.md` §1.3。实现分成两层，iOS 的规则不会经过共享代码落到 Android：

- **共享层只有与平台无关的策略闸门**：`crates/ui-egui/src/input/draw_policy.rs` 的 `DrawPolicy`
  有三档——`AnyInput`（任何输入都可以绘画，默认值）、`PencilOnly`（只有上报压力的触点，即 Apple Pencil，
  能驱动绘制类工具）、`None`（谁都不能绘画）。`Tool::is_drawing` 列出哪些工具算绘制：画笔家族与橡皮擦、
  修复与仿制、填充与渐变、全部选区工具、钢笔与形状工具、文字蒙版。闸门问在每一个入口上：
  `canvas/tool_input.rs` 的 `preflight_tool_event`（所有工具，含读原始指针事件的套索）、液化与
  `dialogs/distort_ui.rs`（内容识别填充、选择并遮住、消失点）、选择并遮住的细化边缘画笔
  （`canvas/select_mask_view.rs`）。被拒绝时状态栏给出提示。
- **松手才触发的点击工具**（油漆桶、魔术橡皮擦、魔棒）不经过按下时的闸门：`input/finger.rs` 记下这次
  按压是不是手指发起的，以及本帧松开的那次按压是不是手指的，`canvas.rs` 据此丢弃手指的点击。
  iOS 外壳为 Pencil 接管指针而合成的那次松开，也被正确算作手指的点击。
- **`None`（iPhone）下还隐藏与置灰**：工具栏不显示绘制类工具（`panels/toolbar.rs` 的
  `visible_sections`），`tool_memory::select` 拒绝选中它们，`menus/state.rs` 把 `select.*`、
  `edit.fill`、`edit.stroke`、`edit.contentAwareFill`、`filter.liquify` 置灰；`set_draw_policy`
  会把当前选中的绘制类工具换成移动工具。
- **手掌不再打断 Pencil**：`PencilOnly` 下，画布的双指仲裁不再因为多出一个触点就取消正在进行的
  笔画（`canvas.rs` 的 `pencil_down`）。
- **共享代码里原先唯一的 iOS 专判已删除**：`stylus::finger_blocks_object_selection` 的
  `cfg!(target_os = "ios")`，对象选择现在由同一套策略闸门管。
- **iOS 外壳自己决定策略**（`apps/photocraft-ios/src/drawing.rs`）：`device_policy()` 用 UIKit 的
  `userInterfaceIdiom` 判断 iPad（`PencilOnly`）或 iPhone（`None`），不看屏幕尺寸。
  `PencilPointer` 在 `raw_input_hook` 里重写 egui-winit 的事件流：手指占着模拟指针时 Pencil 落下，
  就结束手指在画布上的按压并由外壳自己写 Pencil 的指针事件；被系统取消的画布按压会被释放。
  `long_press.rs`（仅 iOS 这一份）让 Pencil 不触发长按菜单。
- **Android 不接入**：`apps/photocraft-android` 没有任何对应代码，策略保持默认的 `AnyInput`。

## 发布与版本

- 三端统一的两段式版本线（`1.2`），`vX.Y` annotated tag 发布，详见 `agent.md` §9–§11。
- GitHub Releases 只放 Android APK 与 macOS DMG；iOS 一律走 App Store Connect。
- Android `versionCode` 每次发布严格递增；iOS `CURRENT_PROJECT_VERSION` 同理。
