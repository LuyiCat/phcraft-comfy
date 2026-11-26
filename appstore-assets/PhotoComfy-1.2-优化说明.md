# PhotoComfy 1.2 优化说明

## 版本信息

- 应用名称：PhotoComfy
- 当前版本：1.2（`CFBundleShortVersionString`；仓库根 `Cargo.toml` 按 Cargo 要求写作 `1.2.0`）
- App Store Connect 构建号：9
- Git tag：`v1.2`
- Bundle ID：`photoComfy`
- 支持设备：iPhone、iPad
- 对照版本：1.1.1

## 1. 内置中文字体

iPadOS 不允许应用读取系统字体，此前画布文字工具（Type 工具）里可直接选用的简体中文字面只有一个。
本版本把 PhotoComfy 中文字体包直接编进应用，19 个家族、21 个字面，全部是允许随应用分发的开源字体：

- **正文多字重**：Noto Sans SC（思源黑体，可变字重 100–900）、Noto Serif SC（思源宋体，200–900），
  以及繁体对应的 Noto Sans TC、Noto Serif TC。
- **楷体与仿宋**：LXGW WenKai Lite（霞鹜文楷，Regular / Medium）、Zhuque Fangsong（朱雀仿宋）。
- **标题与美术字**：Smiley Sans（得意黑）、DouyinSans（抖音美好体）、LXGW Neo XiHei Plus（霞鹜新晰黑）、
  LXGW Marker Gothic（霞鹜漫黑）、ZCOOL KuaiLe / QingKe HuangYou / XiaoWei（站酷快乐体、庆科黄油体、小薇体）、
  Chill Round Gothic（寒蝉全圆体，Regular / Bold）。
- **手写与书法**：Ma Shan Zheng（马善政毛笔楷书）、Long Cang（龙藏体）、Zhi Mang Xing（志莽行书）、
  Liu Jian Mao Cao（流江毛草）、Yozai（悠哉字体）。

字体在“文字工具的字体下拉框”里直接选择即可；用默认的 Inter 输入中文时仍由 Noto Sans CJK SC 自动补齐字形。
许可证文本随应用一并分发（应用包内 `Licences/`）。

## 2. 中文输入

- 文字图层的输入视图补齐 `UITextInput` 协议要求的书写方向方法（`baseWritingDirectionForPosition:inDirection:`、
  `setBaseWritingDirection:forRange:`），避免系统键盘查询时找不到实现。
- 在 iOS 27.0 模拟器上按拼音键盘的调用顺序回放输入（marked text → 候选提交 → 退格 → unmark 后提交），
  画布文字图层最终得到「中文字体书」，系统中文键盘正常弹出。

## 3. 版本号规范

- 自本版本起，版本号统一为两段式（`1.2`、`1.3`……），App Store Connect 中的版本号与 Git tag（`v1.2`）保持一致。
- 历史 tag `v1.1.0`、`v1.1.1` 保持不动。

## 4. 已完成的验证

- iOS 27.0（iPad Pro 11 英寸）模拟器：24 个 CJK 字体家族（含原有的 5 个）样张全部正常出字。
- iOS 27.0 模拟器：拼音输入探针逐步得到「中文字体书」。
- 主机 `photocraft-text` 定向测试（craft_fonts / works_without_craft_fonts 等 11 项）在设置和不设置字体包时均通过；
  `photocraft-text`、`photocraft-io`、`photocraft-engine`、`xtask` 的 Clippy 无警告。
- 无 AIGC 归档、上传与 App Store Connect 接收结果见本次交付记录。

未验证：真实设备上用手指逐键敲击中文软键盘（本机没有辅助功能权限或 Xcode 授权，无法自动点击模拟器键盘）；
Android 设备上的字体显示（Android 产物由 GitHub Actions 在推送 tag 后构建）。

## 面向用户的简短更新说明

PhotoComfy 1.2 内置了 19 款开源中文字体：思源黑体、思源宋体（含繁体）、霞鹜文楷、朱雀仿宋、得意黑、
抖音美好体、霞鹜新晰黑、寒蝉全圆体，以及马善政、龙藏体、悠哉等手写字体和站酷系列美术字，
在文字工具里可以直接选用；同时完善了画布文字的中文输入视图。
