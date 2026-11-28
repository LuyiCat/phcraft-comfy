# PhotoComfy 中文字体包

iPadOS 和 Android 都不让应用读取系统字体文件，Type 工具（画布文字）只能使用构建时嵌入的字体。
上游的 [craft-fonts](https://github.com/storytold/craft-fonts) 只带一个简体中文字面（Noto Sans CJK SC
Regular），所以 PhotoComfy 的移动端构建额外嵌入一个中文字体包：**19 个家族、21 个字面，约 192 MB**。
字体文件不入库（`agent.md` §13），由脚本按锁文件下载并校验。

## 在字体菜单里找到中文字体

字体编进包里还不够：用户要能在字体选项里认出它们。字体包的家族名都是拉丁字母（`Noto Sans SC`、
`Smiley Sans`……），混在几十个其他语种的字体里、旁边只有 `AaBbCc` 样张时，看上去就像“只有英文字体”。
所以中文、日文、韩文界面下（`agent.md` §1.4），字体菜单：

- 在“最近使用的字体”之后列出**本界面文字的字体分组**，标题为“中文字体”“中文字體”“日本語フォント”或
  “한글 글꼴”；分组里的字体按界面语言的名字显示并排序，同时在下方字母序列表里保留原有位置。
- 名字取自字体自己 `name` 表里该语言的记录（得意黑、霞鹜文楷 轻便版、抖音美好体……）；字体没有这种记录时
  （Noto Sans SC、Ma Shan Zheng、ZCOOL KuaiLe 等），取字体包清单里给出的名字（`name` 行，见下文）。
- 覆盖界面文字的字体，样张栏在 `AaBbCc` 旁显示一段界面文字的样张（“中文字体”“中文字體”“日本語”“한글”），
  代替其他界面下的阿拉伯文样张。

分组和样张由界面语言决定，与平台无关：英文等非 CJK 界面下菜单与原来完全一样。哪些字体进分组，嵌入字体看清单
声明的 `scripts`（`Hans`、`Hant`、`Jpan`、`Hang`），系统字体看是否覆盖该文字的代表性字符。

## 字体列表

界面文字的回退仍由 craft-fonts 的 Noto Sans CJK SC 负责；下表是 Type 工具里可以直接选用的家族。

| 家族（英文族名，括号内是简体中文界面下的菜单名） | 字面 | 风格 | 许可证 | 大小 |
|---|---|---|---|---|
| Noto Sans SC（思源黑体） | 可变字重 100–900 | 黑体 | OFL-1.1 | 17 MB |
| Noto Serif SC（思源宋体） | 可变字重 200–900 | 宋体 | OFL-1.1 | 24 MB |
| Noto Sans TC（思源黑体 繁体） | 可变字重 100–900 | 黑体 | OFL-1.1 | 11 MB |
| Noto Serif TC（思源宋体 繁体） | 可变字重 200–900 | 宋体 | OFL-1.1 | 16 MB |
| LXGW WenKai Lite（霞鹜文楷） | Regular、Medium | 楷体 | OFL-1.1 | 26 MB |
| Smiley Sans（得意黑） | Oblique | 窄体斜杠标题字 | OFL-1.1 | 2 MB |
| LXGW Neo XiHei Plus（霞鹜新晰黑） | Regular | 黑体 | IPA-1.0 | 9 MB |
| DouyinSans（抖音美好体） | Bold | 标题黑体 | OFL-1.1 | 2 MB |
| Zhuque Fangsong（朱雀仿宋） | Regular | 仿宋 | OFL-1.1 | 8 MB |
| Yozai（悠哉字体） | Regular | 手写 | OFL-1.1 | 15 MB |
| Chill Round Gothic（寒蝉全圆体） | Regular、Bold | 圆体 | OFL-1.1 | 25 MB |
| Ma Shan Zheng（马善政） | Regular | 毛笔楷书 | OFL-1.1 | 5 MB |
| ZCOOL KuaiLe（站酷快乐体） | Regular | 美术字 | OFL-1.1 | 1 MB |
| ZCOOL QingKe HuangYou（站酷庆科黄油体） | Regular | 标题美术字 | OFL-1.1 | 8 MB |
| ZCOOL XiaoWei（站酷小薇体） | Regular | 宋体感美术字 | OFL-1.1 | 6 MB |
| Long Cang（龙藏体） | Regular | 手写行书 | OFL-1.1 | 5 MB |
| Zhi Mang Xing（志莽行书） | Regular | 手写行书 | OFL-1.1 | 4 MB |
| Liu Jian Mao Cao（流江毛草） | Regular | 手写草书 | OFL-1.1 | 5 MB |
| LXGW Marker Gothic（霞鹜漫黑） | Regular | 马克笔手写黑体 | OFL-1.1 | 3 MB |

除了霞鹜新晰黑采用 IPA 字体许可 1.0，其余都是 SIL Open Font License 1.1，都允许原样嵌入并随应用分发。
每个文件的作者、固定来源与许可证文本列在 [`scripts/photocomfy-fonts.lock`](../scripts/photocomfy-fonts.lock)，
脚本会据此生成字体包目录里的 `ATTRIBUTION.md`。

## 构建如何接入

- `scripts/photocomfy-fonts.sh` 读取锁文件，下载每个文件并核对 SHA-256，写入仓库根目录被 git 忽略的
  `photocomfy-fonts/`，同时生成 `fonts/manifest.txt`（craft-fonts 清单格式）和 `ATTRIBUTION.md`。
  下载地址按顺序尝试：先是 `raw.githubusercontent.com`，不可达时退回 GitHub API 的 raw 媒体类型；
  设置 `GH_TOKEN` 可以避开匿名调用的速率限制。
- `apps/photocraft-ios/ios/build.sh` 和 `apps/photocraft-android/android/build.sh` 在清单缺失时调用该脚本，
  然后设置 `PHOTOCOMFY_FONTS_DIR` 与 `PHOTOCOMFY_FONTS_REQUIRED=1`。
- `crates/text/build.rs` 把字体包的字面追加到 `photocraft_text::CRAFT_FONTS`（排在 craft-fonts 之后），
  因此 `FontDb` 注册、字体菜单、界面按文字脚本的回退都自动生效；桌面和 Web 构建不设置该变量，不受影响。
- 清单行可以带第 9 个字段 `语言=名称;语言=名称`（如 `zh-Hans=思源黑体;zh-Hant=思源黑體`），`build.rs` 把它编进
  `CraftFont::names`，供字体菜单在对应语言的界面下显示（`FontDb::native_family_names`）。字体包脚本按锁文件里的
  `name` 行写出这个字段；craft-fonts 自己的清单只有 8 个字段，其字体只用 `name` 表里的记录。
- 许可证文本随应用分发（`scripts/font-licences.sh`）：iOS 在 `PhotoComfy.app/Licences/`，Android 在 APK 的
  `assets/fonts-licences/`，包括 craft-fonts 和内置的 Inter、JetBrains Mono 的许可证。
- GitHub Actions 的两个 Android 任务在检出 craft-fonts 之后运行同一脚本。

清单里的字体顺序就是嵌入顺序。界面回退取「声明了该文字脚本、且覆盖这个字符的第一个字面」，
所以 Noto 家族排在展示字体之前，繁体界面不会回退到霞鹜文楷。新增字体时沿用这个顺序。

## 更新或新增字体

1. 在 `scripts/photocomfy-fonts.lock` 增改一行：`font` 行的第 2–9 个字段就是清单行
   （family | style | file | scripts | licence | licence file | sha256 | source），其后是下载地址；
   许可证文本各占一行 `text`。文件保持与上游逐字节一致，不要裁剪或改名后重新生成。
2. 字体的 `name` 表里没有简体（`zh-CN`）或繁体（`zh-TW`、`zh-HK`）名称时，为缺少的语言各加一行
   `name | family | 语言 | 名称`（语言用 BCP 47 标签，如 `zh-Hans`、`zh-Hant`），否则该语言的界面下它只能以
   英文族名出现在分组里。
3. 运行 `scripts/photocomfy-fonts.sh`，脚本会重新下载并校验变化的文件并重写清单。
4. 重新构建 iOS 或 Android，按下面的办法在模拟器或设备上核对样张和字体菜单。

## 验证

两个测试开关只存在于开发构建（App Store 上传用的无 AIGC 构建不编译它们）：

- `PHOTOCRAFT_FONT_SPECIMEN=1`：启动时新建文档，为每个嵌入的 CJK 家族写一行中文，
  一张截图就能看到每个字面是否正常出字。
- `PHOTOCRAFT_TYPE_PROBE=1`：新建文档并在画布中央开始编辑文字图层，然后按拼音键盘的调用顺序
  （marked text → 候选提交 → 退格 → unmark 后提交）在真实的 first responder 上回放一段输入，
  `Documents/launch.log` 逐步记录图层里的文字，应以「中文字体书」结束。

```sh
UDID=<模拟器 UDID>
xcrun simctl install "$UDID" <构建产物>/PhotoComfy.app
SIMCTL_CHILD_PHOTOCRAFT_FONT_SPECIMEN=1 xcrun simctl launch --console-pty "$UDID" photoComfy
xcrun simctl io "$UDID" screenshot specimen.png
```

字体菜单本身也要看：没有辅助功能权限时，可以用一个独立的 XCUITest
（`XCUIApplication(bundleIdentifier:)` 加坐标点击）驱动模拟器，实际点开文字工具的字体菜单，确认顶部的
“中文字体”分组、中文名和中文样张，再选一个中文字体输入中文。主机上的对应测试（需要嵌入字体包和
craft-fonts）：`cargo test -p photocraft-ui-egui --lib -- font_picker_tests:: font_preview` 和
`cargo test -p photocraft-text --lib -- native_family_names a_native_name_follows`。

2026-11-26 在 iPad Pro 11 英寸（iOS 27.0）模拟器上的结果：24 个 CJK 家族（含 craft-fonts 的 5 个）全部出字；
拼音探针逐步得到 `z`、`zh` … `中`、`中文` … `中文字体书`，系统中文键盘同时弹出。
主机的 `photocraft-text` 定向测试（`craft_fonts`、`works_without_craft_fonts`）在设置和不设置
`PHOTOCOMFY_FONTS_DIR` 两种情况下都通过。
