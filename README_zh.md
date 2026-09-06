# cargo-slide

> **结合 Typst 排版与 Rust 原生播放器的代码驱动演示文稿引擎**  
> *原生 60 FPS 软件渲染 · 独立单二进制产物 · 硬件加速媒体联动 · 交互式 SQL 图表 · 可扩展动画 Trait*

[English](README.md) | [简体中文](README_zh.md)

---

## 项目概述 (Overview)

`cargo-slide` 是一个用于编写、放映与分发演示文稿的代码驱动命令行工具与运行时。作为 [Apich 工作区 (Apich Organization)](https://github.com/Apich-Organization) 的核心后端基础设施组件之一，它将两个互补的技术结合在一起：

- **[Typst](https://typst.app/) 负责内容排版**：亚秒级快速编译、整洁的标记语法、一流的数学公式排版能力，并通过模块化宏直接编译为高精度矢量图形（SVG）。
- **Rust 负责原生演示播放器**：基于 [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia) 构建的独立运行时，在无需浏览器或 Electron 依赖的前提下以稳定 60 FPS 渲染放映，内置演讲者工具箱、音频混音调度、可交互图表以及单二进制打包功能。

---

## 核心特性 (Key Features)

- **纯 Typst 语言编写**：演示文稿完全由 Typst 标记语言（`slides.typ`）编写，排版直观。只有在需要扩展底层自定义转场或组件动画 Trait 时才需要编写 Rust。
- **清晰的项目结构**：`cargo slide init` 生成的标准工作区仅包含 5 个必要文件，避免冗余配置。
- **自包含单二进制分发**：通过 `cargo slide build`，可将幻灯片矢量图形、排版布局元数据与播放器运行时编译为单个便携式可执行二进制文件（约 14–15 MB）。目标运行机器无需安装 Typst、Node.js、Python 或 Rust 即可直接运行。
- **跨平台窗口与全屏模式**：支持按 `F11` / `F` / 底部 Dock `FULL` 在原生全屏与可调尺寸窗口模式间无缝切换，适配 Linux (EWMH)、Windows 与 macOS。外围黑边区域自动采样并延展幻灯片背景色，在非标准比例屏幕上呈现平滑过渡。原生支持 `16-9`、`16-10`（如 MacBook、ThinkPad 等）、`3-2`（Surface 等）及 `4-3` 画布长宽比。
- **13 种内置页面转场**：`fade`（淡入淡出）、`cut`（硬切）、`slide-left` / `slide-right` / `slide-up` / `slide-down`（平移滑动）、`zoom`（缩放）、`wipe-left` / `wipe-right`（百叶窗擦除）、`iris`（光圈）、`glitch`（RGB 故障抖动）、`cube`（3D 翻转）与 `particles`（伪随机粒子重组）。
- **页内组件分步显现（Steps）**：使用 `#step(order, effect: "...")` 控制元素逐步显现，支持 `fade-in`、`slide-up`、`glitch` 与 `typewriter` 打字机效果。使用空格键 / 左键前进，Backspace / 右键回退。
- **交互式数据图表与 HUD 数据检查器**：
  - 支持直接读取 `.csv`、`.json` 与 `.db` (SQLite) 数据集。
  - 支持在编译/放映期执行内存 SQL（`SELECT ... FROM data WHERE ...`）或链式 DSL（`source -> filter() -> select()`）。
  - 放映时鼠标悬停吸附十字准星与数值指示。
  - 内置 HUD 数据检查器：点击图表或表格即可唤起模态检查窗口，动态切换图表形态（柱状图、折线图、面积图、散点图），应用快速计算（`TOP 5`、`SORT ▼` 降序、`SORT ▲` 升序、`CUM` 累积、`% SHARE` 占比、`MA3` 移动均线），按列排序，多条件检索（文本或数值条件如 `>50`, `<=100`），并支持将筛选后的明细一键导出为 CSV。
- **演讲者工具箱 (Presenter Tools)**：
  - **激光笔**：平滑光斑与模拟衰减拖尾（`L` 键或底部 Dock `LSR`）。
  - **白板涂鸦画笔**：实时自由手绘批注（`P` 键），内置 7 色调色盘（`K` 键，数字键 `1`～`7`），支持平滑贝塞尔混合与按页记忆（`C` / `X` 擦除）。
  - **多通道音频控制**：支持背景音循环播放、换页淡入淡出与悬浮音量调节（滚轮、`+` / `-` 键、Dock 拖动滑块，`M` 静音），音量安全钳位在 0%～100%。
  - **视频播放委托**：在幻灯片中生成带交互热区的视频占位卡片，点击后在后台线程中唤起外部播放器（如 `ffplay`、`mpv` 或系统关联应用）独立播放，支持常规窗口控制、进度定位与自动退出。
- **幻灯片垂直内容溢出检查**：编译时自动比对生成的 SVG 页面数与声明的 `#slide(...)` 数量。一旦内容超出垂直画布限制，编译器将中断并精确输出发生溢出的幻灯片标题与源码行号。
- **完全向量化与多语言排版**：Typst 会将所有西文、中日韩汉字（CJK）、数学公式符号与 Emoji 转化为矢量贝塞尔 `<path>` 轮廓。演示文稿在各操作系统上均具备一致的排版渲染，无需目标机器安装对应字体。HUD 检查器内置点阵字体，适配常用货币符号、排序三角指示、箭头与常用演示图标。
- **无障碍与触控支持**：支持纯键盘全功能导航；无键盘环境支持屏幕底部悬浮触摸 Dock；图表检查器提供高对比度纯文本表格数据视图。
- **双模式日志输出**：默认输出清晰的终端格式化日志；传入 `--log-format json` 或设置 `CARGO_SLIDE_LOG_FORMAT=json` 输出单行机器可读 JSON，方便 CI/CD 自动化集成。

---

## 快速上手 (Getting Started)

### 环境依赖 (Prerequisites)

1. **Rust 工具链**：Rust 1.80+（推荐最新稳定版），可通过 [rustup.rs](https://rustup.rs/) 安装。
2. **Typst 命令行工具**：确保 `typst` 可执行文件在系统 `PATH` 中。可通过包管理器安装（`cargo install --locked typst-cli`、`brew install typst` 或各 Linux 发行版仓库）。
3. *（可选）* **外部媒体播放器**：若需播放嵌入视频，推荐安装 `mpv` 或 `ffplay`。

### 第 1 步：安装 `cargo-slide`

从本地源码安装：
```bash
cargo install --path crates/cargo-slide
```

验证安装是否成功：
```bash
cargo slide --help
```

### 第 2 步：初始化演示文稿工作区

在希望存放幻灯片的目录下运行：
```bash
# 在当前目录直接初始化
cargo slide init

# 或在指定新目录中初始化
cargo slide init my-talk
cd my-talk
```

初始化后的工作区包含恰好 5 个文件：
```text
my-talk/
├── slides.typ          # 幻灯片正文（纯 Typst 语言）
├── theme.typ           # 主题配置（配色方案、字体大小、16:9 比例）
├── slide.typ           # 常用组件宏库（#slide, #step, #chart, #video 等）
├── assets/
│   └── data.csv        # 示例交互式图表数据集
└── .gitignore          # 忽略临时缓存、PDF 导出与二进制产物
```

*（注：如果需要编写自定义 Rust 动画 Trait，可传入 `--rust` 参数：`cargo slide init my-talk --rust`，这将额外生成 `Cargo.toml` 与 `src/main.rs` 脚手架。）*

### 第 3 步：编写幻灯片内容

使用任何文本编辑器打开 `slides.typ`：
```typst
#import "theme.typ": *

#show: slide-theme.with(
  aspect-ratio: "16-9", // "16-9", "16-10"（MacBook / Dell XPS / ThinkPad 等）, "3-2"（Surface / Framework 等）, 或 "4-3"
  theme: "dark"
)

#title-slide(
  title: "现代系统架构设计",
  subtitle: "基于代码驱动的高性能原生演示",
  author: "张三",
  date: "2026",
)

#slide(title: "架构与核心理念", transition: "fade")[
  #cols(
    [
      === 核心优势
      - 纯 Typst 函数式排版标记
      - 原生 60 FPS 矢量渲染
      - 单二进制自包含分发
    ],
    [
      === 经典公式
      $ cal(H) |psi(t) chevron.r = i ħ dif / (dif t) |psi(t) chevron.r $

      #v(0.3cm)
      #badge("量子核心", fill: slide-colors.accent)
    ]
  )
]
```

### 第 4 步：本地放映与热重载

启动原生 60 FPS 放映播放器：
```bash
cargo slide run
```

在编写过程中，可开启开发热重载模式：
```bash
cargo slide dev
```
每当保存 `slides.typ` 时，播放器会自动重编并在保留当前页码的前提下无缝刷新画面。

### 第 5 步：构建独立可执行文件（发布）

完成编写后，执行打包命令：
```bash
cargo slide build
```
该命令会生成一个独立的可执行文件（如 `slides-presentation` 或 `.exe`）。将该单个文件复制到任何演示电脑上即可直接双击全屏放映，受众电脑无需安装 Typst、Rust 或浏览器。

### 第 6 步：导出 PDF 或 SVG

如需导出讲义或归档资料：
```bash
# 导出为高精度 PDF 文档
cargo slide export slides.typ --format pdf -o presentation.pdf

# 导出每一页为独立的 SVG 矢量图
cargo slide export slides.typ --format svg -o exported-svgs/
```

---

## 常见问题解答 (FAQ)

### Q1: 使用 `cargo-slide` 必须学习或编写 Rust 代码吗？
**不需要。** 普通演示文稿完全由 Typst 语言编写（`slides.typ`）。只有在需要使用 Rust Trait 编写底层自定义像素着色器或物理粒子动画时，才需要编写 Rust。

### Q2: `cargo slide init` 生成的项目包含几个文件？
标准模式下恰好生成 5 个文件：
1. `slides.typ`：幻灯片正文。
2. `theme.typ`：16:9 画布尺寸、色彩及字体样式配置。
3. `slide.typ`：封装好的组件宏（`#slide`, `#step`, `#chart`, `#video`, `#audio`, `#callout`）。
4. `assets/data.csv`：示例图表数据集。
5. `.gitignore`：过滤 `.build_tmp/`、`*.cache.csv`、`target/` 及编译生成的二进制文件。

如果附带 `--rust` 参数，则会额外包含用于 Trait 扩展的 `Cargo.toml` 与 `src/main.rs`。

### Q3: 编译为单二进制时，哪些资产直接内嵌？哪些需要单独放在 `assets/`？
- **直接内嵌进独立二进制可执行文件（约 14–15 MB）：**
  - 全部幻灯片的 SVG 矢量图元、排版几何、字形轮廓与 LaTeX 数学公式；
  - 交互式热区元数据、步骤显现逻辑与页面转场属性；
  - 经过预处理的 CSV、JSON、SQLite 图表数据表；
  - `tiny-skia` 软件渲染器、转场算法、激光笔、画笔、音频混音器与 HUD 检查器内核。
- **放置在 `assets/` 目录中随同携带：**
  - **外部视频文件**（`.mp4`, `.webm`, `.mkv`）与外部音频文件。
  - *设计权衡*：如果把几百 MB 的大型视频文件直接压入二进制，会导致可执行文件异常庞大且占用不必要的内存。通过相对路径关联媒体文件并在放映时调度外部播放器，既保持了二进制文件的轻巧紧凑，又规避了解码库依赖。

### Q4: 幻灯片中的视频播放是如何工作的？
Typst 会在页面上绘制视频占位卡片并留下坐标热区。放映过程中点击该卡片时，播放器会在后台线程中唤起外部播放器（优先使用 `ffplay`，其次回退至 `mpv` 或系统默认关联的播放器）在独立窗口中播放。视频播放器具备完整的窗口控制与关闭即返回逻辑，播放结束后自动退出并返回幻灯片，保证演示主线程持续平稳运行。

### Q5: 用户能自定义字体吗？字体未找到有 Warning 吗？
- **文字全矢量化输出**：Typst 在编译时会将所有文字字形（包括中日韩汉字、特殊数学符号、Emoji）完全转换为 SVG `<path>` 矢量轮廓。播放器直接通过 `tiny-skia` 栅格化这些贝塞尔数学曲线，因此目标放映机器完全不需要安装任何对应字体即可保证像素级保真。
- **自定义字体配置**：用户可通过以下三种直观方式自定义字体：
  1. **主题参数配置**：直接在 `#show: slide-theme.with(...)` 中传入 `font: "Inter"`（或字体数组 `font: ("Inter", "PingFang SC")`）以及等宽代码字体 `code-font: "Fira Code"`。
  2. **Typst 原生指令**：在 `slides.typ` 任意位置通过 `#set text(font: "...")` 覆盖指定字体。
  3. **项目级本地字体包打包**：将 `.ttf` 或 `.otf` 字体文件直接放置于工程根目录的 `fonts/` 或 `assets/fonts/` 文件夹下，`cargo-slide` 会自动探测并通过 `--font-path` 参数载入，无需系统级安装即可实现跨平台 100% 确定性渲染。亦可通过 `TYPST_FONT_PATHS` 环境变量指定外部字库路径。
- **字体缺失 Warning 提醒**：若用户指定的字体在当前系统与本地字库目录中均未找到，Typst 编译器会抛出 `warning: unknown font family: ...` 警告。`cargo-slide` 会完整捕获该警告并输出友好的提示信息（包含本地字库打包指引），同时安全降级到系统可用备用字体。

### Q6: 幻灯片垂直溢出防御机制是如何运作的？
在 Typst 中，当内容超出 16:9 垂直画布上限（15.75 cm）时，Typst 会自动隐式分页，导致生成孤儿空白页并破坏幻灯片序号。  
`cargo-slide` 会在编译期核对生成的 SVG 页数与声明的 `#slide` 数量。若发生溢出，会立即定位出溢出发生的幻灯片标题与代码行号：
```text
Typst slide content overflow detected!
The presentation source declares 17 slide(s), but Typst compiled 18 pages (1 extra spillover page(s)).

Overflow location:
  Slide 4 ("Mathematical Typography & Code Syntax Highlighting", line 64) exceeded the vertical 16:9 canvas bounds.
  Spillover content pushed onto compiled page 5.
```
作者可通过微调间距或高度解决此问题，也可通过设置 `CARGO_SLIDE_ALLOW_OVERFLOW=1` 环境变量放行。

### Q7: `cargo-slide` 与 Marp、Slidev、LaTeX Beamer 的客观对比如何？

| 特性对比 | `cargo-slide` | Marp / Slidev | LaTeX Beamer |
| :--- | :--- | :--- | :--- |
| **内核架构** | 原生 Rust (`tiny-skia`) | Web / Electron / Node.js | TeX 引擎 / PDF 查阅器 |
| **交付形式** | 单个约 12 MB 独立二进制 | HTML 文件包 / PDF / App | 静态 PDF |
| **排版保真度** | Typst 矢量字形曲线 | 浏览器 CSS / Web 字体 | LaTeX 原生排版 |
| **数学公式质量** | LaTeX 级精美公式 | KaTeX / MathJax 网页渲染 | 原生 TeX 数学渲染 |
| **放映帧率** | 锁定 60 FPS 稳定刷新 | 取决于 DOM 与浏览器性能 | 静态无转场 |
| **演讲者工具** | 激光笔拖尾、画笔、混音器 | 依赖浏览器插件支持 | 取决于 PDF 阅读器功能 |
| **产物运行依赖** | 无任何外部依赖 | 需要现代浏览器或运行时 | 需要 PDF 阅读器 |
| **客观权衡** | 编辑需 Typst CLI 依赖 | 包体积大、不同浏览器有差异 | 编译慢、语法较为繁复 |

### Q8: 可以在无图形界面的 CI/CD 流水线中使用吗？
可以。`cargo slide build` 与 `cargo slide export` 均可在无头环境中执行。结合 `--log-format json` 参数可以输出标准 JSON 事件流，便于监控构建状态。

---

## 命令行完整参考 (CLI Reference)

全局参数：`--log-format <human|json>`（默认：`human`）或环境变量 `CARGO_SLIDE_LOG_FORMAT=json`。

### `cargo slide init`
```bash
cargo slide init [PATH] [--rust] [--log-format <human|json>]
```
在指定路径初始化幻灯片工作区（默认当前目录 `.`）。可选 `--rust` 参数以附加 Rust Trait 扩展脚手架。

### `cargo slide new`
```bash
cargo slide new <NAME> [--rust] [--log-format <human|json>]
```
新建目录 `<NAME>` 并在其中初始化幻灯片工作区。

### `cargo slide run`
```bash
cargo slide run [FILE] [--animation <NAME>] [--fullscreen]
```
启动原生演示播放器。默认读取 `slides.typ`，默认转场为 `fade`。

### `cargo slide dev`
```bash
cargo slide dev [FILE] [--animation <NAME>]
```
以文件监控模式启动播放器，保存文件即自动编译并刷新画面。

### `cargo slide build`
```bash
cargo slide build [FILE] [-o <OUTPUT>] [--animation <NAME>]
```
将幻灯片编译为自包含的单个独立发行版二进制文件。

### `cargo slide export`
```bash
cargo slide export [FILE] --format <pdf|svg> -o <OUTPUT>
```
将演示文稿导出为单份多页 PDF 文件或分页 SVG 矢量图序列。

---

## 演讲操作与快捷键表 (Presenter Controls)

| 按键 / 操作 | 功能说明 |
| :--- | :--- |
| **空格键** / **回车** / **右方向键** / **下方向键** / **左键点击** | 下一步骤（若当前页步骤已完结则进入下一页） |
| **退格键** / **左方向键** / **上方向键** / **右键点击** | 上一步骤（若当前页无上一步则返回上一页） |
| **PageDown** / **PageUp** | 直接按整页翻页（跳过页内逐步展开） |
| **Home** / **End** | 跳转到第一页 / 最后一页 |
| **输入数字 + 回车** | 快速精准跳转到对应页码 |
| **F11** / **F** / Dock `FULL` | 切换原生全屏与窗口化模式 |
| **L** / Dock `LSR` | 开启 / 关闭激光笔（带平滑拖尾） |
| **P** / Dock `PEN` | 开启 / 关闭白板涂鸦画笔 |
| **K** / Dock `COL` | 弹出 / 隐藏 7 色浮动调色盘 |
| **数字键 1 .. 7** | 选择画笔与激光笔颜色（青/红/绿/黄/紫/白/橙） |
| **C** / **X** / Dock `CLR` | 清除当前幻灯片上的全部画笔笔迹 |
| **滚轮上/下** / **`+` / `-`** | 实时调节主音量（0% ~ 100%） |
| **Dock 音量滑块** | 鼠标点击或按住拖动滑块轨道直接设置音量 |
| **M** / Dock `VOL` | 静音 / 恢复音量 |
| **H** / **?** / Dock `HELP` | 显示 / 隐藏键盘快捷键帮助卡片 |
| **点击图表 / 表格** | 展开内置交互式 HUD 数据检查器模态窗口 |
| **点击视频卡片** | 在独立原生窗口中启动外部视频播放器 |
| **点击超链接** | 在后台默认应用中打开外部网页或本地文件 |
| **Esc** / **Q** | 关闭当前弹窗 / 退出放映 |

---

## 交互式图表与 HUD 数据检查器

### 1. CSV 数据集图表
```typst
#chart(
  data: "assets/data.csv",
  type: "bar",
  title: "系统内存开销对比",
  width: 100%,
  height: 6cm
)
```

### 2. 内存 SQL 查询数据
支持在 CSV、JSON 或 SQLite 数据库上执行标准 SQL：
```typst
#chart(
  data: "assets/telemetry.db",
  sql: "SELECT service, p99_latency FROM traces WHERE p99_latency > 50 ORDER BY p99_latency DESC",
  type: "bar",
  title: "高延迟微服务拓扑"
)
```

### 3. 流水线 DSL 简易查询
```typst
#chart(
  data: "assets/metrics.json",
  dsl: "source -> filter(fps >= 30) -> select(Framework, FPS)",
  type: "line",
  title: "帧率表现分析"
)
```

### 4. 交互式 HUD 数据检查器 (Data Inspector)
在放映过程中点击任意图表或数据卡片，即可呼出内置的数据检查器窗口：
- **动态图表切换**：在柱状图、折线图、面积图与散点图形态间一键无缝转换。
- **快捷分析变换**：快速应用常用数据变换（`ORIG` 原数据、`TOP 5`、`SORT ▼` 降序、`SORT ▲` 升序、`CUM` 累积值、`% SHARE` 百分比占比、`MA3` 三期移动均线）。
- **数据序列开关**：点击系列色块胶囊即可单独显示或隐藏指定数据序列。
- **KPI 指标卡片**：顶部状态栏实时统计并呈现当前可见数据的求和（Sum）、均值（Mean/Avg）、最小值（Min）、最大值（Max）、中位数（Median）与标准差（StdDev）。
- **明细表格浏览**：以网格表格展示底层行数据；点击表头（类别或各系列标题）可按升序/降序排序；支持鼠标滚轮与拖动翻页浏览。
- **多条件检索过滤**：支持文本模糊匹配与数值表达式过滤（如 `>50`、`<=100`、`!=0`）。
- **数值格式切换**：循环切换数值显示精度与格式（`ORIG`、`INT` 整数、`DEC1` 一位小数、`DEC2` 两位小数、`CURR` 货币、`PCT` 百分比）。
- **导出 CSV**：点击 `EXPORT CSV` 按钮，将当前筛选/排序后的数据即时保存为带时间戳的本地 CSV 文件。
- **退出窗口**：按 `Esc` 键或点击右上角 `CLOSE [X]` 返回演示放映。

---

## 使用 Rust Trait 扩展自定义动画

面向希望实现专属渲染算法的开发者：

```rust
use slide_core::animation::{RenderContext, SlideAnimation, SlideSurface};
use slide_player::SlideApp;
use std::time::Duration;

/// 自定义上下卷帘分割转场
pub struct CurtainSplitTransition;

impl SlideAnimation for CurtainSplitTransition {
    fn name(&self) -> &str {
        "curtain-split"
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(500)
    }

    fn render(
        &self,
        ctx: &mut RenderContext,
        from: Option<&SlideSurface>,
        to: &SlideSurface,
        progress: f32,
    ) {
        let t = progress.clamp(0.0, 1.0);
        let (w, h) = (ctx.width, ctx.height);
        let split_y = (h as f32 * t * 0.5) as usize;

        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let pixel = if y < split_y || y >= (h - split_y) {
                    to.get_pixel(x, y)
                } else if let Some(f) = from {
                    f.get_pixel(x, y)
                } else {
                    0
                };
                ctx.buffer[row + x] = pixel;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SlideApp::new("slides.typ")
        .default_animation("curtain-split")
        .register_animation(CurtainSplitTransition)
        .run()?;
    Ok(())
}
```

---

## 模块架构一览 (Project Structure)

```text
cargo-slide/
├── Cargo.toml                      # 根 Workspace 配置
├── crates/
│   ├── slide-core/                 # 核心：数据模型、Typst 编译器桥接、SVG 解析、图表与内存 SQL、Trait、日志引擎
│   ├── slide-player/               # 播放器：tiny-skia 光栅化内核、双窗口管理、音频混音、演讲者工具箱、HUD 检查器
│   ├── slide-theme/                # 模板：默认主题、Typst 常用宏库（#slide, #step, #chart 等）
│   └── cargo-slide/                # 命令行入口：init / new / run / dev / build / export
└── examples/
    └── geek-presentation/          # 17 页完整极客范例：涵盖公式、代码、转场、分步 Steps、图表、媒体与自定义 Trait
```

---

## 维护者与联系方式 (Maintainers & Contact)

`cargo-slide` 作为 [Apich 工作区 (Apich Organization)](https://github.com/Apich-Organization) 的核心后端基础设施组件之一进行开发与维护。

- **维护者 (Maintainer)**：Xinyu Yang ([Xinyu.Yang@apich.org](mailto:Xinyu.Yang@apich.org))
- **组织机构 (Organization)**：Apich Organization ([info@apich.org](mailto:info@apich.org))
- **项目仓库**：[https://github.com/Apich-Organization/cargo-slide](https://github.com/Apich-Organization/cargo-slide)

---

## 开源许可证 (License)

本项目遵循 [GNU Affero 通用公共许可证 v3.0 或更高版本](LICENSE) (`AGPL-3.0-or-later`) 开源。

