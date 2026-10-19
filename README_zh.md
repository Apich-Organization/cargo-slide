# cargo-slide

> **结合 Typst 排版与 Rust 原生播放器的代码驱动演示文稿引擎**  
> *原生 60 FPS 软件渲染 · 独立二进制、`.slide` 归档与纯 Rust WebAssembly 三重交付 · 交互式 SQL 图表 · 可扩展动画 Trait*

[English](README.md) | [简体中文](README_zh.md)

---

## 项目概述 (Overview)

`cargo-slide` 是一个用于编写、放映与分发演示文稿的开源命令行工具与演示运行时，主要面向习惯使用代码组织内容的研究人员与开发者。作为 [Apich 工作区 (Apich Organization)](https://github.com/Apich-Organization) 的基础组件之一，它结合了两项互补的技术：

- **[Typst](https://typst.app/) 负责排版**：亚秒级快速编译、简洁的标记语法、高质量数学公式排版，并通过函数式宏直接编译为矢量图形（SVG）。
- **Rust 负责原生演示播放**：基于 [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia) 构建的独立运行时，在无需浏览器或 Electron 依赖的前提下以稳定 60 FPS 呈现画面，提供演讲者工具箱、音频调度混音、可交互 SQL 图表以及三种灵活的交付分发模式。

---

## 三大交付范式 (Three Delivery Paradigms)

`cargo-slide` 针对不同演讲与分发场景提供三种交付方式：

| 交付范式 | 适用场景 | 产物形态与体积 | 目标机依赖 |
| :--- | :--- | :--- | :--- |
| **独立二进制 (Standalone Binary)** | 专用演播电脑、严肃演讲汇报 | 单个可执行文件（约 14–15 MB） | 无任何依赖（开箱即播） |
| **通用归档包 (`.slide` Archive)** | 邮件分发、即时通讯传输、U盘随带 | 压缩归档包（约 1 MB，LZMA2 压缩） | 通过 `slide-viewer` 播放 |
| **WebAssembly CSR 网页端** | 在线网页预览、团队文档集成、在线投屏 | 纯静态站点（严格零内联 JavaScript） | 任何现代浏览器 |

---

## 核心特性 (Key Features)

- **纯 Typst 语言编写**：文稿完全由 Typst 标记语言（`slides.typ`）书写，语法直观。仅在需要编写自定义底层转场算法或光栅着色器时才需要引入 Rust。
- **轻量工作区脚手架**：`cargo slide init` 生成恰好 5 个必要文件，不产生多余配置文件。
- **Typora 风格所见即所得桌面编辑器 (`slide-editor`)**：
  - 实时矢量渲染画布，支持在自然画卷流中连续预览幻灯片。
  - **行内就地编辑 (In-Place Edit)**：点击幻灯片内容即可就地修改 Typst 标记源码，内置防抖实时重编译更新。
  - **分类插入栏**：快捷插入基础图文（段落、各级标题、列表、代码块、数学公式）、多媒体与数据（表格、SQL 图表、视频、音频、超链接）、演播标记（演讲备忘笔记、块注释、标注框、网格分栏）。
  - **大纲与缩略图侧边栏**：提供 16:9 矢量缩略图预览卡片与紧凑层级列表，方便快速翻页定位。
  - **演讲配速模型与备忘可视化**：明确区分演讲者备忘笔记（`SPEAKER NOTE`）与文档注释（`[COMMENT]`），支持中英双模字数统计（西文字词 + CJK 汉字）、预计用时与单页信息密度监控。
  - **文稿健康度体检**：自动排查本地素材路径有效性、排版文字密度与编译器潜在告警。
- **通用演示查看器 (`slide-viewer`)**：
  - 现代化自适应 GUI 启动引导界面，支持按 `F` / `F11` 进入真全屏模式。
  - 自动扫描当前目录中的演示文档，内置 22 页完整示例演讲文稿。
  - 原生文件选择对话框（`O` / `Ctrl+O`），支持随时读取 `.slide`、`.typ` 源码或 `deck.json`。
  - 跨平台一键系统安装与后缀关联（`I` 键或 `slide-viewer install` / `cargo slide install-viewer`），支持 Linux (`.desktop`、MIME)、macOS (`.app` 包) 与 Windows (注册表)。
- **演讲者工具箱 (Presenter Tools)**：
  - **4 种白板画笔**：普通实心笔 (`Pen`)、荧光高亮笔 (`Highlighter`，3.5× 笔宽，正片叠底不遮文字)、霓虹笔 (`Neon`，双层发光光晕)、导向箭头笔 (`Arrow`，运笔末端自动生成几何箭头)。
  - **14 色调色板与微调**：数字键 `1`～`0` 快速切换，笔宽分级调节（`[` / `]`），笔触撤销（`U` / `Ctrl+Z`），按页记忆与一键清屏（`C` / `X`）。
  - **激光笔**：高对比度红色光斑配合物理模拟衰减拖尾（`L` 键）。
  - **音频引擎**：背景音乐循环播放、跨页平滑淡入淡出、主音量滑块调节与自动淡出的 Toast 状态提示。
  - **外部视频联动**：点击视频卡片即可在独立原生窗口中唤起外部播放器（`mpv`、`ffplay` 或系统默认播放器），不阻塞演示主线程。
- **交互式 SQL 图表与 HUD 数据检查器**：
  - 直接读取 `.csv`、`.json` 或 `.db` (SQLite) 数据集。
  - 支持执行内存 SQL（`SELECT ... WHERE ...`）或管道链式 DSL 计算。
  - 放映时点击任何图表或数据表即可唤起 HUD 检查器：动态切换图表形态（柱状图、折线图、面积图、散点图），应用快速分析预设（`TOP 5`、`SORT ▼` 降序、`SORT ▲` 升序、`CUM` 累积、`% SHARE` 占比、`MA3` 均线），数值条件筛选（如 `>50`），并支持将筛选后的明细一键导出为 CSV。
- **页面内容垂直溢出防护**：编译时比对生成的矢量页面数与实际声明的 `#slide(...)` 数量。一旦内容高度超出纵向限制，编译器将准确定位溢出的幻灯片标题与源代码行号。
- **通用渲染稳定性与全矢量排版**：
  - 原生 GUI 界面（`slide-editor`、`slide-viewer`）与 CLI 终端日志统一采用标准通用 Unicode / ASCII 字符标识（如 `[OK]`、`[WARN]`、`[ERR]`、`[INFO]`、`+ Note` 等），避免在未安装彩色 Emoji 字体的 Linux 环境或极简容器中出现方块乱码（Tofu）；
  - Typst 排版将文稿内容（西文、中日韩汉字、数学公式与特殊符号）编译解析为高精度贝塞尔 `<path>` 矢量轮廓，保障跨平台渲染一致。
- **13 种内置转场与页内分步显现**：支持 `fade`、`cut`、`slide-left`、`slide-right`、`slide-up`、`slide-down`、`zoom`、`wipe-left`、`wipe-right`、`iris`、`glitch`、`cube` 与 `particles`。支持 `#step(order, effect: "...")` 页内元素逐步呈现。
- **文稿加密与完整性保护 (Presentation Encryption)**：
  - **硬件 PIV 令牌模式 (YubiKey / 智能卡)**：将文稿解密绑定至硬件私钥（`9d` 密钥管理槽位），并通过硬件签名私钥（`9c` 数字签名槽位）对 SHA3-512 完整性摘要进行签名验证。演播需插入物理令牌并验证 PIN 码。
  - **软件双钥密钥库模式 (密码口令 / BIP-39 助记词)**：面向无物理硬件令牌场景，将非对称 P-256 接收密钥与签名密钥封装于受 AES-256-GCM 保护的密钥库中。基于高强度密码或可选 12/24 词 BIP-39 助记词（`--mnemonic-words 12|24`），经由 PBKDF2-HMAC-SHA512 (600,000 次迭代) 与 HKDF-SHA256 安全派生保护密钥。
  - **内存解密防落盘与运行时加固**：文稿与多媒体素材在 RAM 中直接解包并由 `MemoryAssetStore` 调度，全程不写入物理磁盘。解密数据段调用 `mlock` 锁定内存以防止写入交换分区，并在销毁时通过 `zeroize` 覆写清零。运行时限制核心转储（`RLIMIT_CORE = 0`），在 Linux 上设置 `PR_SET_DUMPABLE = 0` 阻止内存转储与未授权附加，在 macOS 上启用 `PT_DENY_ATTACH` 阻止调试器附加。
  - **掩码输入与重试反馈机制**：终端命令行与 GUI 弹窗输入密码、PIN 及助记词时均进行掩码显示。密码或 PIN 验证失败时提供最多 3 次重试机会并实时提示剩余尝试次数，避免误输入直接中断。RSA/ECDSA 比对采用严格恒定时间比对杜绝时序泄漏。
  - **可视化编辑器集成 (`slide-editor`)**：原生编辑器完整支持加密 `.slide` 幻灯片的读取、编辑与保存，打开加密文稿时自动唤起图形凭据验证弹窗。导出对话框支持直接打包为加密文稿（口令、12/24 词助记词或 YubiKey PIV）。
  - **Web 模式受保护部署与会话令牌**：在 WebAssembly 导出与托管时（`cargo slide build/serve/export --format wasm --encrypt`），文稿编译为加密信封，访问者在浏览器内输入口令或助记词后在纯 Rust WASM 客户端完成解密，并生成带有可配置存活时长（`--token-ttl <秒>`，默认 24 小时）的会话令牌保存在 `sessionStorage` 中，避免明文文稿泄露在静态服务器或 CDN 上。

---

## 快速上手 (Getting Started)

### 环境要求 (Prerequisites)

1. **Rust 工具链**：Rust 1.80+（推荐最新稳定版），可通过 [rustup.rs](https://rustup.rs/) 安装。
2. **Typst 命令行工具**：确保 `typst` 命令可在终端执行。可通过包管理器安装（`cargo install --locked typst-cli`、`brew install typst` 或 Linux 发行版仓库）。
3. *（可选）* **外部媒体播放器**：如需演示外置视频，推荐安装 `mpv` 或 `ffplay`。

### 第 1 步：安装 `cargo-slide` 与 `slide-viewer`

通过本地源码安装：
```bash
cargo install --path crates/cargo-slide
cargo install --path crates/slide-viewer
```

或将 viewer 注册到系统：
```bash
slide-viewer install
```

### 第 2 步：初始化演示文稿

```bash
cargo slide init my-talk
cd my-talk
```

生成的项目目录结构整洁简洁：
```text
my-talk/
├── slides.typ          # 幻灯片正文内容（纯 Typst 语言）
├── theme.typ           # 主题配置（配色方案、字体大小、16:9 画布比例）
├── slide.typ           # 内置组件宏库（#slide, #step, #chart, #video 等）
├── assets/
│   └── data.csv        # 示例交互式图表数据集
└── .gitignore          # 忽略编译缓存与输出文件
```

*（注：若需扩展自定义 Rust 动画 Trait，可传入 `--rust` 参数：`cargo slide init my-talk --rust`。）*

### 第 3 步：编写与放映

使用任何文本编辑器打开 `slides.typ`，随后运行：

```bash
# 启动 Typora 风格所见即所得桌面编辑器实时预览与就地编辑
cargo slide edit

# 启动原生 60 FPS 播放器放映
cargo slide run

# 或启动带文件监控的实时热重载开发模式
cargo slide dev

# 全面体检文稿健康度（元素密度、素材路径有效性、演讲配速）
cargo slide check
```

### 第 4 步：发布、打包与导出

```bash
# 1. 编译为自包含单个可执行二进制（约 14 MB）
cargo slide build

# 2. 打包为 .slide 归档文件（约 1 MB，LZMA2 压缩）
cargo slide pack -o presentation.slide

# 3. 导出为 Leptos WebAssembly CSR 纯静态网页
cargo slide export --format wasm -o dist-web/

# 4. 使用内置轻量 HTTP 服务器本地托管并自动打开浏览器
cargo slide serve --open

# 5. 导出为多页 PDF 讲义或独立 SVG 矢量图
cargo slide export --format pdf -o presentation.pdf
cargo slide export --format svg -o exported-svgs/

# 6. 打包与加密演示文稿
# 硬件 PIV 模式 (需要已连接的 YubiKey / 智能卡):
cargo slide pack slides.typ --encrypt
# 软件密码口令模式 (强制密码强度检查：至少 10 位且包含 3 类字符):
cargo slide pack slides.typ --encrypt --auth passphrase
# 软件 BIP-39 助记词模式 (支持指定 12 或 24 词):
cargo slide pack slides.typ --encrypt --auth mnemonic --mnemonic-words 12
# 编译带加密保护的独立可执行文件:
cargo slide build slides.typ --format binary --encrypt --auth passphrase
# 导出受保护的 Web 静态站点并配置会话 Token 存活时长 (纯 Rust WASM 客户端解密):
cargo slide serve --encrypt --auth passphrase --token-ttl 86400
cargo slide export --format wasm --encrypt --auth mnemonic -o dist-web/

# 7. 跨平台多架构编译（支持 Windows 与 Linux，x86_64 与 aarch64）
# 使用 Zig 工具链在 Linux 交叉编译 Windows x86_64 单文件可执行程序：
cargo slide build slides.typ --target x86_64-pc-windows-gnu --cross-tool zig
# 使用 Cross 容器引擎交叉编译 Linux aarch64 (ARM64) 可执行程序：
cargo slide build slides.typ --target aarch64-unknown-linux-gnu --cross-tool cross
# 一键编译全部主流桌面架构可执行文件：
cargo slide build slides.typ --all-targets
# 批量交叉编译整个 cargo-slide 核心工程及配套工具链：
./scripts/cross-build.sh --target all
```

---

## 演示操作与快捷键一览 (Presenter Controls)

| 按键 / 交互 | 功能说明 |
| :--- | :--- |
| **Space** / **Enter** / **Right** / **Down** / **鼠标左键** | 前进至下一个分步动画（若无分步则切换至下一页） |
| **Backspace** / **Left** / **Up** / **鼠标右键** | 回退至上一个分步动画（或上一页） |
| **PageDown** / **PageUp** | 直接按整页翻页 |
| **Home** / **End** | 快速跳转至第一页 / 最后一页 |
| **数字 + Enter** | 快速定位跳转至指定页码 |
| **F11** / **F** / 底部 Dock `FULL` | 切换真全屏与自适应窗口模式 |
| **L** / 底部 Dock `LSR` | 开启/关闭激光笔（带物理衰减拖尾） |
| **P** / 底部 Dock `PEN` | 开启/关闭白板涂鸦批注 |
| **T** | 循环切换画笔类型（普通笔、荧光笔、霓虹笔、方向箭头） |
| **[** / **]** | 调小 / 调大画笔笔触宽度（2px, 4px, 8px, 14px） |
| **数字键 1 .. 0** | 快速选择 14 种预设高对比度色彩 |
| **U** / **Ctrl+Z** | 撤销上一笔画笔笔迹 |
| **C** / **X** / 底部 Dock `CLR` | 清空当前幻灯片上的所有手绘标注 |
| **鼠标滚轮** / **`+` / `-`** | 调整主音频音量（0% 至 100%） |
| **M** / 底部 Dock `VOL` | 静音 / 恢复播放 |
| **H** / **?** / 底部 Dock `HELP` | 显示/关闭快捷键帮助悬浮窗 |
| **O** / **Ctrl+O** | 唤起系统文件选择器加载演示文档 (.slide, .typ, .json) |
| **I** | 一键将 slide-viewer 安装至系统桌面与应用列表 |
| **点击图表 / 表格** | 唤起 HUD 交互式数据检查器 |
| **点击视频卡片** | 在独立后台窗口中启动外部视频播放器 |
| **点击超链接** | 访问外部网络或预览本地文本内容 |
| **Esc** / **Q** | 关闭弹窗模态 / 退出放映 |

---

## 交互式图表与 SQL 查询示例

### 1. CSV 数据源图表
```typst
#chart(
  data: "assets/data.csv",
  type: "bar",
  title: "运行时内存占用评测",
  width: 100%,
  height: 6cm
)
```

### 2. 内存 SQL 查询
通过标准 SQL 语句直接筛选 CSV、JSON 或 SQLite 数据：
```typst
#chart(
  data: "assets/telemetry.db",
  sql: "SELECT service, p99_latency FROM traces WHERE p99_latency > 50 ORDER BY p99_latency DESC",
  type: "bar",
  title: "高延迟微服务拓扑"
)
```

### 3. 链式管道 DSL
无需复杂 SQL 时可使用管道链式变换：
```typst
#chart(
  data: "assets/metrics.json",
  dsl: "source -> filter(fps >= 30) -> select(Framework, FPS)",
  type: "line",
  title: "帧率性能横向对比"
)
```

---

## 技术方案对比 (Technical Comparison)

| 关键特性 | `cargo-slide` | Marp / Slidev | LaTeX Beamer |
| :--- | :--- | :--- | :--- |
| **核心渲染引擎** | 原生 Rust (`tiny-skia`) / 纯 WASM | 网页浏览器 / Electron / Node.js | TeX 引擎 / PDF 阅读器 |
| **主要交付形态** | 独立二进制 / `.slide` 包 / WebAssembly | HTML 静态包 / PDF / 打包 App | 静态 PDF 文件 |
| **字体排版精度** | Typst 原生贝塞尔矢量轮廓 | 浏览器 CSS / Web 字体 | LaTeX 预置字体 |
| **数学公式质量** | Typst LaTeX 级别公式排版 | KaTeX / MathJax | 原生 TeX 数学排版 |
| **帧率与流畅度** | 稳定 60 FPS 光栅渲染 | 受浏览器 DOM 性能影响 | 静态无转场动画 |
| **演播交互能力** | 4 种笔刷、激光拖尾、音频混音、SQL 检查器 | 依赖外部浏览器插件 | 取决于具体 PDF 软件 |
| **目标环境依赖** | 产物独立，无运行时依赖 | 需安装浏览器或 Node 环境 | 需安装 PDF 阅读器 |

---

## 仓库工程结构 (Workspace Structure)

```text
cargo-slide/
├── Cargo.toml                      # 工作区配置
├── crates/
│   ├── slide-core/                 # Typst 编译桥接、SVG 解析、图表计算/SQL、LZMA2 压缩打包
│   ├── slide-player/               # 原生播放引擎：tiny-skia 渲染、窗口事件、音频混音、演播工具、HUD
│   ├── slide-editor/               # Typora 风格所见即所得桌面编辑器（实时矢量预览、就地编辑、大纲与导出）
│   ├── slide-viewer/               # 通用桌面演示文稿播放器（带交互式 GUI 启动台与多平台安装器）
│   ├── slide-web/                  # 纯 Rust Leptos 0.7 CSR Web 播放器（严格零内联 JavaScript）
│   ├── slide-theme/                # 内置排版主题、模板与 Typst 组件宏（#slide, #step, #chart）
│   └── cargo-slide/                # 命令行主入口（init, new, run, dev, edit, check, build, pack, serve, export）
└── examples/
    ├── geek-presentation/          # 22 页官方示例演讲（全景展示所有新增特性）
    ├── dist-web/                   # 预生成的 Leptos CSR 静态网页包
    └── slides.slide                # 预生成的官方参考 .slide 演示包（LZMA2 压缩）
```

---

## 维护者与联络方式 (Maintainers)

`cargo-slide` 作为 [Apich 工作区](https://github.com/Apich-Organization) 的基础设施模块持续开发与维护。

- **维护者**: Xinyu Yang ([Xinyu.Yang@apich.org](mailto:Xinyu.Yang@apich.org))
- **组织**: Apich Organization ([info@apich.org](mailto:info@apich.org))
- **项目仓库**: [https://github.com/Apich-Organization/cargo-slide](https://github.com/Apich-Organization/cargo-slide)

---

## 开源许可协议 (License)

本项目遵循 [GNU Affero General Public License v3.0 或更高版本](LICENSE) (`AGPL-3.0-or-later`) 开源许可。
