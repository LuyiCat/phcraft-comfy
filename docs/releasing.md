# 发布流程（PhotoComfy / PhotoCraft）

> 本页是 [`agent.md`](../agent.md) 第 9、10、10.1 节发布规范的摘要与操作细节。两者不一致时，以 `agent.md` 为准。
> 旧的上游 `release` 分支发布流程已废弃：本仓库不使用长期 `release` 分支，发布一律基于 `main` 上的 `vX.Y.Z` annotated tag。

## 发布模型

- 唯一长期分支是 `main`。发布通过 `main` 上的 annotated Git tag 进行，tag 统一为 `vX.Y.Z`（SemVer，不带平台前缀、日期或构建号后缀；禁止 `ios-v1.0.3`、`release-1.0.3` 等命名）。
- 所有平台共用同一条版本线：`Cargo.toml`、iOS `MARKETING_VERSION`、Android `versionName` 使用同一个不带 `v` 的版本号。
- iOS `CURRENT_PROJECT_VERSION` 与 Android `versionCode` 是商店/分发构建序号，每次上传严格递增，不写入 Git tag。
- 分发渠道（agent.md §10.1）：
  - **Android**：GitHub Release 上传签名 APK，命名 `PhotoComfy-<version>-android-arm64.apk`。
  - **macOS**：GitHub Release 上传 universal DMG，命名 `PhotoCraft-<version>-macos-universal.dmg`，必须 Developer ID 签名、hardened runtime、公证并 stapling。
  - 同一个 Release 还须包含覆盖全部产物的 `SHA256SUMS.txt`。
  - **iOS**：按 agent.md 第 11 节上传 App Store Connect，必须使用 `PHOTOCOMFY_DISABLE_AIGC=YES` 在编译期完整移除 AIGC。

## 发布流程（摘要）

1. 确认本地 `main` 与远程同步、工作区干净。
2. 在一个独立的版本提交中统一更新版本号与构建序号（根 `Cargo.toml` 与 `Cargo.lock`、`PhotoComfy.xcodeproj` 的 `MARKETING_VERSION`/`CURRENT_PROJECT_VERSION`、Android `versionName`/`versionCode`），可用 `cargo xtask version set <version>` 修改工作区版本。
3. 按 agent.md 第 7 节完成最小必要验证，并完成本次发布平台的打包和安装/启动冒烟验证；不额外运行全量测试。
4. 推送 `main`。
5. 在版本提交上创建 annotated tag 并经仓库所有者同意后推送：

   ```sh
   git tag -a v1.0.4 -m "Release v1.0.4"
   git push origin v1.0.4
   ```

6. 由 tag 或受控手动工作流构建发布产物；Release 说明只列出实际构建并验证过的平台。
7. tag 创建后不得移动、重用或改指其他提交；版本有误时递增版本号并创建新 tag。

## macOS 打包与签名验证

`packaging/macos/package.sh` 构建 universal（Apple silicon + Intel）DMG，这也是 agent.md 第 8 节规定的 macOS 发布包门禁。正式产物使用 Developer ID 签名并公证；本地无证书时脚本 ad-hoc 签名并跳过公证，只能用于本地检查，不得冒充正式发布包：

```sh
packaging/macos/package.sh --arch aarch64     # 快速，仅本机架构
packaging/macos/verify.sh --arch aarch64      # 按用户下载方式校验签名、hardened runtime 与公证票据
```

Android 正式发布使用固定、可恢复并妥善备份的 release keystore；keystore 与密码只保存在受控本地环境或 GitHub Secrets。发布前验证 APK 签名、覆盖安装、应用启动以及本次改动路径；每次发布递增 `versionCode`（agent.md §10.1）。

## 发布工作流

- `.github/workflows/release.yml` 由 `v*` tag push 或手动 `workflow_dispatch`（可传 `version` 覆盖，仅用于测试运行）触发：桌面产物由共享可复用工作流构建，Android 任务交叉编译并用 secrets 中的固定 release keystore 签名、校验 `PhotoComfy-<version>-android-arm64.apk`；全部产物与覆盖它们的 `SHA256SUMS.txt` 汇总为一个 draft GitHub Release，draft 经仓库所有者检查后手动 Publish。桌面构建失败时，Android-only 兜底任务仍会发布带校验和的 APK draft。
- Android 签名 secrets（`ANDROID_KEYSTORE_BASE64`、`ANDROID_KEYSTORE_PASSWORD`、`ANDROID_KEY_PASSWORD`）是仓库级 secrets；桌面签名 secrets 经共享工作流的 `release` environment 传入。
- 推送发布 tag 前，确认上述 secrets 已配置、keystore 已在仓库外备份，且新签名可覆盖安装旧版本（agent.md §10.1）。
- 上游桌面（Windows/Linux/FreeBSD/Web）的打包与发布说明见上游仓库 [storytold/photocraft 的 `docs/releasing.md`](https://github.com/storytold/photocraft/blob/main/docs/releasing.md)。相关打包脚本仍保留在 `packaging/` 中随上游演进，但发布产物线以本仓库 tag 发布为准。

## 应用内更新检查

macOS 桌面端（PhotoCraft）在启动时的后台线程检查一次 GitHub latest-release endpoint，无周期检查或自动重试。Preferences › General › Automatically Check for Updates 可关闭启动检查；Help › Check for Updates 手动触发一次。检查使用 15 秒超时和 1 MiB 响应上限，不需要 GitHub token，也不发送文档或个人数据。自动检查的网络失败保持安静，手动检查窗口中可见；并发请求共享一次检查。HTTPS 按操作系统信任库校验证书与主机名。

只有比已安装工作区版本更新的已发布稳定版本才触发提示；tag 必须是语义化版本（`v0.8.0` 或 `0.8.0`）。Draft 和 prerelease 不触发提示。Flatpak 构建同样检查 Release；仓库托管的安装通过包管理器更新。Web/headless 前端没有更新通道。发布较新的稳定 GitHub Release 即可被发现。
