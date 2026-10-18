# DiagCraft

[English](../../README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Português](README.pt.md) · **简体中文**

**连接你的创意。** 一款原生、本地运行的开源图表编辑器，提供七种界面语言和面向 AI 智能体的 API。

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

![DiagCraft 英文界面](../images/interface-en.png)

## 下载

应用程序和对应的 SHA-256 校验和均可在[发布页面](https://github.com/Kevin-FR/DiagCraft/releases)获取。

| 平台 | 安装包 |
| --- | --- |
| macOS 13+，Apple Silicon | ZIP 中的 `.app` 应用；CLI 单独打包 |
| Windows x86_64 | 包含应用和 CLI 的便携式 ZIP |
| Linux x86_64 | 包含应用和 CLI 的 amd64 `.deb` 软件包或 `.tar.gz` 压缩包 |

**0.6.1 为实验版本**，新增画布内直接编辑文字。Mac 安装包使用临时签名（ad hoc），
尚未通过 Apple 公证。Windows 安装包未使用发布者证书签名。
请参阅[安装指南](../installation.md)。

## 功能

- 直接在块内编辑文字：双击或按 F2/Enter，选择文字后使用浮动格式工具栏。点击块外完成编辑；Escape 取消。
- 界面支持英语、法语、德语、西班牙语、意大利语、葡萄牙语和简体中文。
- 可选的 GitHub 更新检查、SHA-256 下载校验以及手动安装。
- 形状、文本、图表、八个符号库和九个模板。
- 可连接到形状或保留自由端点的连接线，支持连接点和可编辑的折点。
- 多页文档、纸张预设、自定义页面尺寸以及适应绘图内容。
- 填充、渐变、图案、轮廓、箭头和阴影。
- 多选、对齐、分布、辅助线、取消手势和撤销历史。
- 触控板导航、按选项卡组织的紧凑功能区，以及可调整大小并记住设置的侧栏。
- 通过 ⌘K / Ctrl+K 打开命令面板，通过 F1 打开可搜索的键盘帮助。
- 可编辑的 `.draw` 文件、带版本历史的本地项目图库，以及 SVG/PDF 导出。
- 自动根据文本调整形状尺寸，测量字体并在界面、SVG 和 PDF 中共用文本布局。
- 崩溃后的自动本地恢复副本，不覆盖原始文件。
- 本地 CLI 和 MCP 服务器：检查、验证和原子 JSON 补丁。

文档格式 **v7** 和智能体 API **v1** 分别独立编号，与应用版本无关。
v1–v6 格式的文件会在打开时于内存中迁移；保存前不会修改原始文件。

绘图无需账户或 API 密钥。MCP 服务器在你选择的本地文件夹内工作；应用不会连接 AI 模型。

在**文件 → 语言与设置…**中选择界面语言。绘图中的文本保持不变。
Mac 安装包仅包含 `arm64` 代码，使用 macOS 系统库：**无需 Rosetta**。

详细指南目前为法语：[语言与更新](../langues-et-mises-a-jour.md)、[命令](../commandes.md)、
[功能区与侧栏](../interface.md)、[绘图恢复](../recuperation.md)、[文本适应](../ajustement-texte.md)和[版本管理](../versioning.md)。

[块内容](../contenu-blocs.md) (FR).

## 开发

安装 [Rust](https://rustup.rs/) 和 Python 3.11 或更高版本，然后运行：

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

Debian/Ubuntu 的系统编译依赖见[开发指南](../development.md)。

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## 项目结构

| 目录 | 用途 |
| --- | --- |
| `crates/core` | 文档模型、几何、页面、历史、SVG/PDF、图库和智能体 API |
| `apps/diagcraft` | 原生 egui/eframe 应用 |
| `apps/diagcraft-cli` | 无图形界面的控制台程序 |
| `assets` | 字体、品牌资源和许可声明，附来源与校验和 |
| `docs` | 架构、格式、开发、安全和版本发布文档 |
| `packaging` | 安装包构建与各目标平台的依赖许可收集 |
| `examples` | 演示文档和格式兼容性测试文件 |

界面、CLI 和 MCP 共用同一引擎；引擎不依赖 egui。
请参阅[架构](../architecture.md)和[依赖选择](../dependencies.md)。

## 参与贡献

计划中的工作和验收标准见 [ROADMAP.md](../../ROADMAP.md)。分组、旋转、图片导入、PNG 导出、
避开障碍物的连接线路由，以及集成安装程序仍待开发。SVG 文本的显示取决于查看器所用的字体。

请阅读 [CONTRIBUTING.md](../../CONTRIBUTING.md)。使用智能体操作绘图时，请遵循
[智能体协议](../AGENTS-DRAW.md)。漏洞报告方式见 [SECURITY.md](../../SECURITY.md)。
这些贡献者文档目前为法语。

## 许可与来源

项目代码采用 [MIT 许可](../../LICENSE)。依赖和字体保留各自的许可，相关声明随安装包分发。
[THIRD_PARTY.md](../../THIRD_PARTY.md) 列出来源与署名信息，
[发布审计](../licensing.md)说明检查内容及其局限。
