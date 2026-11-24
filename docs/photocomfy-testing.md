# PhotoComfy 适配代码的测试与验证

本仓库遵循「最小必要验证」原则（`agent.md` §7）：测试只验证本次改动，不跑全工作区套件、
不重复证明上游核心功能。本文说明适配代码在各层级怎么测、发版前做哪些冒烟验证。

## 1. 主机单元测试（最快的一层）

共享 crate 的行为改动，直接跑覆盖该行为的既有定向测试，例如：

```sh
# 触摸手势 / 竖屏界面 / 界面缩放相关的定向测试
cargo test -p photocraft-ui-egui --lib -- portrait_ touch_scale touch_interface

# 交互回放（kittest harness，按 iOS/Android 事件投递方式模拟点击与笔画）
cargo test -p photocraft-ui-egui --lib -- ios_tap_flow
```

配套静态检查：

```sh
cargo fmt --all -- --check
cargo clippy -p <受影响的 crate> --all-targets --locked -- -D warnings
```

与本次改动无关的失败属于上游既有问题，记录证据即可，不扩大修改范围。

## 2. Android 构建与模拟器验证

构建（release 签名需要 `PHOTOCOMFY_ANDROID_KEYSTORE` 指向固定 release keystore）：

```sh
apps/photocraft-android/android/build.sh out/android
```

产物核对：

```sh
BT=~/Library/Android/sdk/build-tools/<最新版>
"$BT/apksigner" verify --print-certs out/android/PhotoComfy.apk   # 证书指纹须与上一版一致
"$BT/aapt2" dump badging out/android/PhotoComfy.apk               # versionName / versionCode / arm64-v8a / 非 debuggable
```

模拟器冒烟清单（每次发版必做，在 arm64 模拟器上从上一正式版覆盖安装）：

1. 覆盖安装成功（签名一致时无需先卸载，保留首次安装时间）。
2. 应用启动，新建/打开文档正常。
3. 双指左滑撤销一层、右滑重做一层，视图不跟随移动。
4. 双指平移、捏合缩放正常；两指按住 1 秒以上不弹长按菜单。
5. 浮动窗口可以遮盖触屏快捷栏，被遮盖处点按不触发快捷栏按钮。
6. 竖屏（或窄窗口）下工具条在画布上方、面板抽屉在画布下方，界面缩放稳定不抖动。

## 3. iOS 验证（Xcode 图形界面）

按 `agent.md` §1.2，iOS 只在 Xcode 中验证：选择 iPad 模拟器或真机 **Run**，
需要查看模拟设备窗口时使用 Xcode 自带的 DeviceHub。发版前用
**Any iOS Device (arm64)** 执行 **Product ▸ Archive**，核对构建日志的
`AIGC transport: disabled (explicit override)`、归档版本号与签名团队后再上传。

## 4. 发版冒烟验证

正式发布（`agent.md` §10）除上述平台验证外，还要：

- 核对产物命名：`PhotoComfy-<version>-android-arm64.apk`、`SHA256SUMS.txt` 覆盖全部资产。
- GitHub Release 只上传本次实际构建并验证过的平台的产物。
- Release 说明分别列出各平台的改动与验证结果；未构建/未验证的平台如实写明。

## 现有测试资产位置

- `crates/ui-egui/src/ios_tap_flow_tests.rs` — 按 iOS 事件流回放的点击/手势测试，含竖屏布局与缩放收敛回归。
- `crates/ui-egui/src/quick_actions_tests.rs` — 触屏快捷栏层级（浮动窗口遮盖）测试。
- `crates/ui-egui/src/prefs/prefs_ui/tests.rs` — 界面缩放、触摸开关的定向测试。
- `tests/interactions/` — 交互测试数据。
- `docs/integration-testing/` — 上游的集成测试框架说明。
