<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**老派、纯 ASCII 的 Windows 系统工具箱**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · **简体中文** · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma**（土耳其语，意为“鹤嘴镐”）是一个约 2.7 MB 的单文件 `kz.exe`，在终端里清理、探索和维护
Windows 电脑，带有复古的绿屏风格。无需安装程序，没有后台服务，不收集遥测数据。
界面支持 11 种语言（English、Türkçe、简体中文、繁體中文、日本語、한국어、Deutsch、Français、Polski、
Українська、Русский），默认使用你的 Windows 显示语言；可用 `kz config lang de` 切换。

## 功能

| | |
|---|---|
| **清理** | 100 多种缓存、临时文件、日志和回收站，包括 AI 工具、游戏启动器和显卡驱动残留（`clean`）· Windows.old、升级文件、内存转储、休眠文件、旧的还原点（`deepclean`）· 带树状图和最大文件视图的磁盘浏览器（`analyze`）· 文件类型（`types`）· 项目构建产物（`purge`）· 旧安装包（`installer`）· 重复文件（`dupes`）· 空文件夹和失效快捷方式（`empty`）· 使用痕迹（`privacy`） |
| **程序和驱动** | 卸载程序及其残留（`uninstall`）· 用 winget 更新程序（`upgrade`）· 来自 Windows 更新的驱动更新（`drivers`）· 启动程序（`startup`）· 推广应用和 Windows 广告（`debloat`）· 换新电脑备份（`backup`） |
| **系统** | 健康报告（`health`）· 含 GPU 和电池信息的实时监视，以及能解释进程为何运行的进程管理器（`status`）· 维护：DNS、图标缓存、SSD TRIM、DISM、SFC（`optimize`）· 修复：网络、Windows 更新、应用商店、系统文件（`repair`）· 电源计划和电池（`power`）· 还原点（`restore`）· HTML 报告（`report`）· 磁盘速度测试（`bench`）· 自动清理（`schedule`） |
| **网络** | IP、延迟、DNS 基准测试与切换、测速、已保存的 Wi-Fi（`net`）· 哪个程序占用哪个端口（`ports`） |
| **实用** | 哪个程序占用着某个文件（`lock`）· 保持屏幕常亮（`awake`）· 擦屏模式（`cleanscreen`） |
| **kazma** | 受保护的路径（`whitelist`）· 你自己的清理规则（`rules`）· 资源管理器右键菜单（`context`）· 语言和主题（`config`）· 自我更新（`update`）· 日志（`history`）· 卸载 kazma（`remove`） |

每个列表都支持 `/` 搜索，也可以用鼠标操作（单击选中，再次单击打开，滚轮滚动）。
有新版本发布时，菜单会提示你。

它很快。在 NTFS 驱动器上以管理员身份运行时，`analyze` 和 `types` 会直接读取驱动器的主文件表（MFT）
（这正是 WizTree 的秘诀），几秒钟就能得出整个驱动器的大小。否则会并行计算大小；`dupes` 先比较大小，
再比较首尾各 64 KB，最后才用 BLAKE3 对整个文件计算哈希（83,000 个文件 / 5.9 GB 约 7 秒）。

## 安装

Windows 10/11，x64 和 ARM64。无需管理员权限。

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

这会安装到 `%LOCALAPPDATA%\Programs\kazma`，并将 `kz` 添加到 PATH。其他方式：

- **Scoop**：`scoop bucket add kazma https://github.com/hhasanguclu/kazma`，然后 `scoop install kazma`
- **winget**：每个版本的清单都会生成在 [`packaging/winget`](packaging/winget) 中；获准收录后：`winget install hhasanguclu.kazma`
- **手动**：从[最新版本](https://github.com/hhasanguclu/kazma/releases/latest)下载 `kz.exe`

> 该 exe 暂未进行代码签名，因此 SmartScreen 可能会发出警告：点击 **更多信息 -> 仍要运行**。
> 通过 SignPath 签名的准备工作已完成，见 [docs/SIGNING.md](docs/SIGNING.md)。

**卸载**：`kz remove`（删除 exe、它的 PATH 条目以及 `%LOCALAPPDATA%\kazma`）。

## 命令

| 命令 | 作用 |
|---|---|
| `kz` | 交互式菜单 |
| `kz clean` | 临时文件、崩溃转储、浏览器 / 应用 / 开发者缓存、回收站 |
| `kz deepclean` | Windows.old、升级安装文件、ESD 文件、内存转储、休眠文件、较旧的还原点（需管理员） |
| `kz analyze [path]` | 磁盘占用浏览器：`t` 树状图，`b` 最大文件，`s` 排序，`/` 搜索，`w` 谁在用；会解释特殊文件（hiberfil、WSL 磁盘、iPhone 备份...）；删除的内容进入回收站 |
| `kz types [path]` | 哪类文件占用空间（视频、图片、压缩包...）以及主要扩展名 |
| `kz purge [paths...]` | 构建产物：`node_modules`、`target`、`bin/obj`、`.venv`、`__pycache__`、... |
| `kz installer [paths...]` | “下载”和桌面上的旧安装文件（`.exe`、`.msi`、`.msix`、`.iso`） |
| `kz dupes [paths...]` | 重复文件（大小 -> 部分哈希 -> BLAKE3，全部并行） |
| `kz empty [paths...]` | 空文件夹、失效快捷方式和 0 字节文件（移到回收站） |
| `kz privacy` | 最近文件、“运行” / 资源管理器历史、剪贴板、浏览器历史和 Cookie（需手动勾选） |
| `kz uninstall` | 卸载经典程序、Microsoft Store 应用和 Scoop 应用，然后清理残留文件夹和注册表项（先备份）；`--leftovers` 查找以其他方式卸载的应用的残留 |
| `kz upgrade` | 通过 winget 更新已安装的程序 |
| `kz drivers` | 来自 Windows 更新的驱动更新；`--installed` 按时间列出第三方驱动 |
| `kz startup` | 开启/关闭启动程序（与任务管理器使用相同的标志，完全可逆） |
| `kz debloat` | 按固定列表移除预装的推广应用，关闭广告；`--restore` 可撤销 |
| `kz backup [folder]` | 换新电脑工具包：winget 应用列表、驱动、程序列表、kazma 设置；`kz backup restore FOLDER` 重新安装 |
| `kz health` | 磁盘 SMART、电池损耗、蓝屏、错误、更新、Defender、防火墙 |
| `kz status` | 实时 CPU、GPU、内存、磁盘、网络、电池；`enter` 解释进程为何运行，`x` 结束进程，`m` 按内存排序 |
| `kz power` | 电源计划（`set saver / balanced / high / ultimate`）、`battery` 报告、`wake` 唤醒源、`hibernate off / reduced / full` |
| `kz restore` | 系统还原点：`list`、`create "desc"`、`enable`、`open`（Windows 自带向导） |
| `kz report` | HTML 系统报告：硬件、健康、启动项、程序、驱动 |
| `kz bench [path]` | 磁盘速度测试：顺序和 4K 随机读写，绕过系统缓存 |
| `kz net` | IP 和延迟；`dns` 基准测试，`dns set cloudflare`；`speed` 测速；`wifi [--show]` |
| `kz ports [filter]` | 哪个程序在监听哪个端口；`--kill PORT` 将其关闭 |
| `kz optimize` | 失效的启动项（先备份）、重启“开始”/搜索、刷新 DNS、图标缓存，到期时执行 SSD TRIM / DISM / SFC |
| `kz repair [fix]` | `network`、`update`（重置 Windows 更新）、`store`、`dism`（+SFC）、`time`、`printer`；先创建还原点（需管理员） |
| `kz schedule weekly` | 自动运行 `kz clean --yes`（`daily`、`weekly`、`monthly`、`off`、`status`） |
| `kz lock <path>` | 哪个程序打开了某个文件或文件夹 |
| `kz awake [minutes]` | 保持屏幕和电脑唤醒 |
| `kz cleanscreen [seconds]` | 忽略输入的黑屏，方便擦拭屏幕 |
| `kz whitelist` | kazma 绝不删除的路径（`--add`、`--remove`、`--edit`） |
| `kz rules` | 你为 `kz clean` 定制的清理规则（`--edit` 用记事本打开） |
| `kz context install` | 在资源管理器右键菜单中添加“Analyze with kazma”（`uninstall` 将其移除） |
| `kz config` | `kz config` ↑↓ enter · `kz config lang de`（auto、en、tr、zh-CN、zh-TW、ja、ko、de、fr、pl、uk、ru），`kz config theme amber`（green、amber、white、blue） |
| `kz update` | 从 GitHub 更新 kazma 本身（校验和验证） |
| `kz history` | kazma 所有操作的日志 |
| `kz remove` | 卸载 kazma |

所有会删除或安装内容的命令都支持 `--dry-run`（`-n`），可先预览。
在**管理员**终端（以管理员身份运行）中运行，才能安装驱动、执行需要管理员权限的维护任务，
以及清理 Windows 更新和系统临时缓存。

## 安全性

- `clean` 只处理已知的、可重新生成的缓存和临时文件位置；不到 24 小时的临时文件会保留。
- `purge` 只在文件夹旁边存在项目标记文件（`package.json`、`Cargo.toml`、`.csproj`...）时才删除该文件夹。
- 驱动器根目录、你的用户配置文件夹、`Windows`、`Program Files`、`AppData` 等文件夹被强制禁止删除。
- `analyze`、`installer`、`dupes` 和 `uninstall` 的残留都会移到**回收站**。
- `dupes` 始终在每组中至少保留一个文件。
- `drivers` 只安装 **Windows 更新**提供的驱动，并且会先创建**还原点**。
- `startup` 只切换任务管理器的开/关标志；`debloat` 只移除固定列表中的应用，`--restore` 可撤销其设置更改。
- `update` 会用发布版本的 `SHA256SUMS.txt` 校验下载内容。
- `deepclean` 通过 Windows 自带的磁盘清理删除 Windows.old 和升级文件，并始终保留最新的还原点。
- `repair` 会重命名 Windows 更新文件夹而不是删除，并且会先提议创建还原点。
- MFT 读取器只读取驱动器，从不写入。
- 注册表残留和失效的启动项在删除前会导出到 `%LOCALAPPDATA%\kazma\backups`。
- `uninstall --leftovers` 的建议项默认不勾选，并且会移到回收站。
- **白名单**：`%LOCALAPPDATA%\kazma\whitelist.txt` 中的路径永远不会被删除。如果该文件存在但无法读取，kazma 将拒绝运行。
- 每个操作都有日志记录（`kz history`）；没有终端时，除非传入 `--yes`，否则不会删除任何内容。

## 构建

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

代码也可以在 Linux/macOS 上构建以便开发；仅限 Windows 的部分放在 `cfg(windows)` 之后。
发布时，提高 `Cargo.toml` 中的 `version` 并推送到 `main`：CI 会在 Windows 上测试，真实运行每个只读命令，
然后发布 `vX.Y.Z`，附带 `kz.exe`、一个 zip 包和 `SHA256SUMS.txt`，并更新 Scoop / winget 清单。

## 截图

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health` 和 `startup` 截取自真实的 Windows 运行环境（GitHub Actions，Windows Server 2025）。
其他界面来自同一份代码在终端中使用演示数据运行的结果，所以路径看起来像 `/home/user/demo`。</sub>

## 致谢

**灵感。** kazma 的灵感来自 [tw93](https://github.com/tw93) 的 [Mole](https://github.com/tw93/Mole)，
一个 macOS 深度清理工具箱。kazma 用 Rust 从零编写，与它没有共享任何代码。

**翻译。** 除英语和土耳其语外，每种语言都以 JSON 文件的形式存放在 [`locales/`](locales) 中，
将英文文本映射到译文。非常欢迎纠错：编辑文件并发起 pull request
（`python tools/i18n_extract.py --missing de` 会列出某种语言尚缺的内容）。

**库。** 感谢以下项目的作者和维护者：

| Crate | 用途 | 许可证 |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | 终端 UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 终端输入和颜色 | MIT |
| [clap](https://github.com/clap-rs/clap) | 命令行 | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU、内存、磁盘、进程 | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | 并行扫描 | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | 重复文件哈希 | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | 回收站 | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell JSON、翻译 | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | 错误处理 | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | `tr!` 翻译宏 | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe 图标和版本信息 | MIT |

**平台。** Microsoft 的 [winget](https://github.com/microsoft/winget-cli)（程序更新）、Windows Update
Agent API（驱动更新）、[Scoop](https://scoop.sh)（分发）以及
[SignPath Foundation](https://signpath.org)（为开源项目提供免费代码签名）。

**美术。** 横幅采用经典的 [FIGlet](http://www.figlet.org) “standard” 字体样式；截图使用
[DejaVu Sans Mono](https://dejavu-fonts.github.io) 字体渲染。

**作者。** [hhasanguclu](https://github.com/hhasanguclu)。欢迎提交错误报告、想法和 pull request。

## 许可证

[MIT](LICENSE)
