<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/media/readme/title-dark.png">
    <img src="docs/media/readme/title-light.png" alt="Horizon macOS Offline Edition" width="320">
  </picture>
</p>

<p align="center">
  <b>Horizon macOS 独立离线版（Apple Silicon M 芯片 + Intel 双架构 · 26.3）</b><br>
  基于纯 Rust 从零重写的 Minecraft: Java Edition 26.3 原生客户端 —— 无需安装 Java、免启动器正版门禁、一键导入 <code>horizon-26.3-resources.zip</code> 即刻离线畅玩。<br>
  <sub>Pure-Rust native Minecraft: Java Edition 26.3 client for macOS (Apple Silicon &amp; Intel) with instant offline play and 1-click <code>resources.zip</code> import.</sub>
</p>

<p align="center">
  <a href="#horizon-macos-独立离线版简体中文"><b>简体中文安装与使用教程</b></a> · <a href="#horizon-macos-offline-edition-english-guide"><b>English Installation &amp; Usage Guide</b></a>
</p>

<p align="center">
  <img src="docs/media/readme/hero-lookout.webp" alt="Horizon 游戏实机画面 / Horizon Gameplay" width="100%">
</p>

---

# Horizon macOS 独立离线版（简体中文）

本项目分支（`macos-offline`）基于上游 [Horizon](https://github.com/horizon-rust/horizon) 引擎定制，体验全面对齐 [horizon-android](https://github.com/samgum/horizon-android)：将游戏客户端直接打包为开箱即用的独立原生 **`Horizon.app`**，同时支持 **Apple Silicon（M 系列芯片）** 与 **Intel（英特尔芯片）**，支持直接导入现成的 `horizon-26.3-resources.zip` 离线畅玩。

## 一、核心特性

- **原生 Rust 驱动的 Java 版 26.3 完整体验**：底层基于 `winit` + `wgpu`（macOS 原生 Metal 图形后端）与 CoreAudio，无需安装庞大耗电的 JVM 或官方启动器，完整支持生存/创造/极限模式、原版存档直读、红石与生物逻辑、局域网联机、离线及正版多人服务器、资源包、光影包与模组系统。
- **一键 / 拖拽导入 `horizon-26.3-resources.zip` 即玩**：
  - 自动识别 `~/Downloads` 下的资源压缩包，支持**一键导入**、**原生访达文件选择框**、**直接拖入窗口**与**命令行参数导入**四种方式。
  - 自动兼容压缩包内顶层带 `26.3/horizon-resources.json` 目录或根目录直放 `horizon-resources.json` 两种打包格式，自动过滤 macOS `__MACOSX/` 与 `._*` 隐藏文件，实时展示解压进度条，解压完成后自动拉起游戏主菜单，后续启动零等待秒进。
- **双模式账号中心（免登录离线模式 + 微软正版共存）**：
  - 默认启用 **离线模式（`Player`）**，打开应用零登录门禁直达主界面，随时可在主菜单左上角自定义 `1~16` 位离线玩家名进行单机与离线服测试。
  - 同时支持通过 **Microsoft OAuth 2.0 设备码** 登录正版账号（自动复制验证码并拉起浏览器），敏感令牌采用 **macOS 系统钥匙串（Keychain）** 加密保存并支持自动续期、自定义 Azure `Client ID` 与调试 `Session JSON` 导入。
- **Mac 全芯片原生兼容（M 芯片 + Intel 芯片）**：
  - 修复了 `x86_64-apple-darwin` 在 Rosetta 2 及部分 macOS 环境下主线程 `GuestStackUnavailable` 退出的上游隐患，Intel 真机与 M 芯片均稳定运行。

---

## 二、完整下载与安装教程（必读）

### 第 1 步：选择对应的安装包

前往本仓库的 **[Releases 页面](../../releases)** 下载最新版本的安装包（提供 `.dmg` 镜像与免挂载 `-app.zip` 压缩包）：

| 文件名 | 适用设备 | 说明 |
| --- | --- | --- |
| **`horizon-macos-arm64.dmg`** | **Apple Silicon（M1 / M2 / M3 / M4 全系列）** | **M 芯片首选**，纯原生 `arm64` 编译，体积更精简 |
| **`horizon-macos-x64.dmg`** | **Intel（英特尔处理器 Mac）** | 原生 `x86_64` 编译（亦可在 M 芯片上通过 `arch -x86_64` 测试 Rosetta 2） |
| **`horizon-macos-universal.dmg`** | **通用版（通吃 M 芯片 + Intel）** | 通过 `lipo` 合并双架构胖二进制，适合放入移动硬盘在多台不同芯片 Mac 间共用 |

双击下载好的 `.dmg` 文件，在弹出的窗口中将 **`Horizon.app`** 拖拽到右侧的 **`Applications`（应用程序）** 文件夹即可完成安装。

---

### 第 2 步：首次打开的 Ad-hoc 自签名放行（隐私与安全性 → 仍要打开）

> **为什么会弹出安全拦截提示？**
> 本项目的离线测试构建使用 macOS 原生 **Ad-hoc 自签名（`codesign --sign -`）**，未绑定付费的 Apple Developer ID 商业公证证书。因此从浏览器下载 `.dmg` / `.zip` 后首次启动时，macOS 的 Gatekeeper 会携带隔离标记（Quarantine）并弹出拦截提示（如 *“未打开 Horizon / Apple 无法验证 Horizon 是否包含恶意软件”*）。**只需按下方任一方法放行一次，今后即可永久正常双击打开：**

#### 方法 A：系统设置图形界面放行（无须敲命令）

- **如果你使用的是 macOS 15 (Sequoia) / macOS 26 及更新系统**：
  1. 在「应用程序」中双击一次 `Horizon.app`，弹出 *“未打开 Horizon”* 提示框时，点击 **「完成」（Done）** —— **注意不要点“移到废纸篓”**。
  2. 打开系统左上角  菜单 → **「系统设置」（System Settings）** → 左侧点击 **「隐私与安全性」（Privacy & Security）**。
  3. 向下滚动到底部的 **「安全性」（Security）** 区域，你会看到提示：*“已阻止使用 Horizon，因为来自身份不明的开发者”*。
  4. 点击右侧的 **「仍要打开」（Open Anyway）** 按钮，在弹出的二次确认窗口中再次点击 **「仍要打开」**，并通过 Touch ID 或输入开机密码确认即可！

- **如果你使用的是 macOS 12 / 13 / 14 (Monterey / Ventura / Sonoma)**：
  1. 打开「访达（Finder）」进入「应用程序」目录，找到 `Horizon.app`。
  2. **按住键盘 `Control` 键点击（或鼠标右键点击）`Horizon.app`**，在右键菜单中选择 **「打开」（Open）**（注意首次不要直接左键双击）。
  3. 在弹出的确认框中点击 **「打开」（Open）**，此后即可像普通软件一样直接双击启动。

#### 方法 B：终端一行命令秒过（最省事推荐）

打开「终端（Terminal）」，粘贴执行以下一行命令清除下载隔离属性：

```bash
xattr -cr /Applications/Horizon.app
```

执行完毕后直接双击 `/Applications/Horizon.app`，即可零弹窗秒开！

---

### 第 3 步：导入 `horizon-26.3-resources.zip` 游戏资源包

本仓库不包含任何受版权保护的 Mojang 游戏资产。首次启动 `Horizon.app` 时若尚未导入资源，会进入资源导入引导页，支持以下 **4 种方式**直接导入你准备好的 `horizon-26.3-resources.zip`（内部为 `26.3/horizon-resources.json` 或根目录直放 `horizon-resources.json` 均可）：

1. **一键自动识别导入（最方便）**：只要你的资源包（如 `horizon-26.3-resources.zip`）位于系统的 `~/Downloads`（下载）文件夹中，界面会直接显示蓝色主按钮 **「一键导入 Downloads/horizon-26.3-resources.zip」**，点击即可立即开始解压！
2. **文件选择框导入**：点击 **「选择资源压缩包 (.zip)…」**，在弹出的原生访达窗口中选中任意磁盘位置的 `.zip` 资源包。
3. **直接拖入窗口**：直接从访达（Finder）按住 `horizon-26.3-resources.zip` 拖进 `Horizon` 窗口松手即可自动开始解压。
4. **终端命令行直传**：
   ```bash
   /Applications/Horizon.app/Contents/MacOS/horizon-client --resources ~/Downloads/horizon-26.3-resources.zip
   ```

> **提示**：解压过程会在窗口内实时显示百分比与文件计数进度条；进度到达 `100%` 后**无需手动重启，会自动拉起并进入 Minecraft 26.3 游戏主界面**！此后每次打开 `Horizon.app` 都会直接秒进主界面。
>
> *(如果你还没有 `horizon-26.3-resources.zip`，可在任意安装并运行过一次正版 Minecraft: Java Edition 26.3 的电脑上执行 `cargo run --release -p horizon-prepare -- --prepare`，将生成的 `26.3` 目录压缩为 `horizon-26.3-resources.zip` 即可跨机使用。)*

---

### 第 4 步：离线畅玩、自定义玩家名与正版账号切换

1. **默认离线模式**：进入主界面后默认处于离线测试模式（默认玩家名 `Player`），可直接点击「单人游戏」创建世界，或在「多人游戏」连接局域网 / 离线模式（`online-mode=false`）服务器。
2. **修改离线玩家名 / 登录微软正版**：
   - 点击主界面左上角的 **`● 离线: Player [账号]`** 胶囊按钮，打开内置的 **「账号与资源中心」** 弹窗：
   - **修改离线名字**：在输入框中输入 `1~16` 位英文字母、数字或下划线（`_`），按回车或点击 **「启用离线模式」** 即可实时生效（无需重启游戏）。
   - **登录微软正版**：先复制你的 Azure 公共客户端 `Client ID` 点击「粘贴 Azure Client ID」（或通过环境变量 `HORIZON_MSA_CLIENT_ID` 传入），再点击 **「微软正版设备码登录」** —— 应用会自动将 8 位设备码复制到剪贴板并打开浏览器授权页，授权完成后自动换取正版会话并存入 macOS 钥匙串（Keychain `dev.horizon.macos.msa`）。
   - **更换资源包**：弹窗底部还提供 **「导入 / 更换游戏资源包 (.zip)…」** 按钮，后续更新资源包也可一键覆盖。

---

## 三、数据与存档目录说明

独立版 `Horizon.app` 的所有运行数据统一存放在 macOS 标准应用支持目录：

- **游戏主目录（存档 `saves/`、设置 `options.txt`、资源包 `resourcepacks/`、模组 `horizon-mods/`、截图 `screenshots/`）**：
  `~/Library/Application Support/horizon/`
- **已解压的 26.3 核心资源目录**：
  `~/Library/Application Support/horizon/resources/26.3/`
- **离线/正版账号配置与系统钥匙串项**：
  配置文件位于 `~/Library/Application Support/horizon/horizon_accounts.json`，正版刷新令牌加密存储于 macOS「钥匙串访问」的 `dev.horizon.macos.msa` 服务项下。

## 四、从源码本地编译与打包

```bash
# 1. 添加 Apple Silicon 与 Intel 双编译目标
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# 2. 编译双架构 Release 客户端
cargo build --release --locked --target aarch64-apple-darwin -p horizon-official --bin horizon-official
cargo build --release --locked --target x86_64-apple-darwin -p horizon-official --bin horizon-official
cp -f target/aarch64-apple-darwin/release/horizon-official target/aarch64-apple-darwin/release/horizon-client
cp -f target/x86_64-apple-darwin/release/horizon-official target/x86_64-apple-darwin/release/horizon-client

# 3. 打包生成 arm64 / x64 / universal 三套 .app、.dmg 与 .zip
packaging/macos/package_game_app.sh target/aarch64-apple-darwin/release/horizon-client 26.3 dist arm64
packaging/macos/package_game_app.sh target/x86_64-apple-darwin/release/horizon-client 26.3 dist x64
packaging/macos/package_game_app.sh target/aarch64-apple-darwin/release/horizon-client 26.3 dist universal target/x86_64-apple-darwin/release/horizon-client
```

---
---

# Horizon macOS Offline Edition (English Guide)

This branch (`macos-offline`) packages the pure-Rust [Horizon](https://github.com/horizon-rust/horizon) engine (*Minecraft: Java Edition 26.3*) as a standalone native **`Horizon.app`** for both **Apple Silicon (`arm64`)** and **Intel (`x86_64`)** Macs, mirroring the zero-gate offline workflow and 1-click resource-archive import of [horizon-android](https://github.com/samgum/horizon-android).

## 1. Key Features

- **Native Pure-Rust Java Edition 26.3 Engine**: Powered by `winit` + `wgpu` (native Metal backend) + CoreAudio — no JVM and no external launcher required. Supports survival, creative, and hardcore modes, vanilla save compatibility, LAN worlds, offline & online multiplayer servers, resource packs, shaders, and Horizon mods.
- **1-Click & Drag-and-Drop `horizon-26.3-resources.zip` Import**:
  - Import your prepared `horizon-26.3-resources.zip` via **1-click `~/Downloads` auto-detection**, **native Finder file dialog**, **window drag-and-drop**, or **CLI arguments**.
  - Automatically supports both nested (`26.3/horizon-resources.json`) and root-level (`horizon-resources.json`) zip structures, strips macOS `__MACOSX/` / `._*` metadata, displays a real-time extraction progress bar, and launches straight into the Main Menu as soon as extraction reaches `100%`.
- **Dual-Mode Account Center (Instant Offline Mode + Microsoft Genuine Sign-In)**:
  - Starts in **Offline Mode (`Player`)** out of the box with zero login gate. Click the top-left account pill on the Main Menu anytime to customize your `1..=16` character offline username.
  - Also supports **Microsoft OAuth 2.0 Device-Code** sign-in for online-mode servers, sealing tokens inside the **macOS Keychain** (`dev.horizon.macos.msa`) with automatic token renewal, custom Azure `Client ID`, and debug `Session JSON` paste support.
- **Apple Silicon & Intel Ready**: Resolves the `x86_64-apple-darwin` main-thread `GuestStackUnavailable` startup issue under Rosetta 2 and Intel macOS.

---

## 2. Step-by-Step Installation Guide

### Step 1: Download & Install the App

Download the package matching your Mac from the **[Releases page](../../releases)**:

| Package | Target Mac | Notes |
| --- | --- | --- |
| **`horizon-macos-arm64.dmg`** | **Apple Silicon (M1 / M2 / M3 / M4)** | **Recommended for M-series Macs** — native `arm64` build |
| **`horizon-macos-x64.dmg`** | **Intel (`x86_64`) Macs** | Native Intel build (also testable on Apple Silicon via `arch -x86_64`) |
| **`horizon-macos-universal.dmg`** | **Universal (Apple Silicon + Intel)** | Fat Mach-O binary combining both `arm64` and `x86_64` slices |

Open the downloaded `.dmg` and drag **`Horizon.app`** into the **`Applications`** folder.

---

### Step 2: Allow the Ad-hoc Signed App on First Launch (Privacy & Security → Open Anyway)

> **Why does macOS show an unverified developer prompt?**
> Community offline builds are signed **ad-hoc (`codesign --sign -`)** without a paid Apple Developer ID notarization certificate. When downloaded via a web browser, macOS attaches a quarantine attribute and blocks the very first launch (*"Horizon Not Opened / Apple could not verify Horizon is free of malware"*). **Approve it once using either method below and it will open normally forever:**

#### Option A: Via macOS System Settings (GUI)

- **On macOS 15 (Sequoia) / macOS 26 and newer**:
  1. Double-click `Horizon.app` in **Applications** once. When the *"Horizon Not Opened"* dialog appears, click **Done** *(do NOT click Move to Trash)*.
  2. Open **System Settings** → **Privacy & Security**, then scroll down to the **Security** section.
  3. Next to *"Horizon was blocked to protect your Mac"*, click **Open Anyway**.
  4. Click **Open Anyway** again in the confirmation dialog and authenticate with Touch ID or your Mac password.

- **On macOS 12 / 13 / 14 (Monterey / Ventura / Sonoma)**:
  1. Open Finder → **Applications**, then **Control-click (or right-click) `Horizon.app`** and choose **Open** from the context menu.
  2. Click **Open** in the dialog. Subsequent launches work with a normal double-click.

#### Option B: One-Line Terminal Command (Fastest)

Run the following command in **Terminal** to strip the browser quarantine flag:

```bash
xattr -cr /Applications/Horizon.app
```

Then simply double-click `/Applications/Horizon.app` — it will launch immediately without any Gatekeeper prompt!

---

### Step 3: Import `horizon-26.3-resources.zip` (First Launch Only)

This repository distributes **no** copyrighted Mojang assets. On first launch, `Horizon.app` opens an interactive setup screen offering **4 ways** to import your prepared `horizon-26.3-resources.zip`:

1. **1-Click Downloads Import (Easiest)**: If `horizon-26.3-resources.zip` is in your `~/Downloads` folder, simply click the highlighted **`Import Downloads/horizon-26.3-resources.zip`** button!
2. **Native File Picker**: Click **`Choose Resources (.zip)…`** and select your `.zip` archive anywhere on disk.
3. **Drag & Drop**: Drag `horizon-26.3-resources.zip` from Finder directly onto the `Horizon` window.
4. **Command Line**:
   ```bash
   /Applications/Horizon.app/Contents/MacOS/horizon-client --resources ~/Downloads/horizon-26.3-resources.zip
   ```

Once extraction reaches `100%`, Horizon automatically launches straight into the Minecraft 26.3 Main Menu. Subsequent launches skip extraction and start instantaneously.

*(Need to generate `horizon-26.3-resources.zip`? On any computer with an official Minecraft: Java Edition 26.3 installation that has been launched once, run `cargo run --release -p horizon-prepare -- --prepare` and zip the resulting `26.3` directory.)*

---

### Step 4: Offline Play, Custom Username & Microsoft Sign-In

1. **Instant Offline Mode**: Horizon defaults to Offline Mode (`Player`) — jump straight into Singleplayer or join LAN / offline-mode (`online-mode=false`) Multiplayer servers without signing in.
2. **Customize Offline Name or Sign In with Microsoft**:
   - Click the **`● Offline: Player [Account]`** pill in the top-left corner of the Main Menu to open the built-in **Account & Resources Center**:
   - **Offline Name**: Type any `1..=16` character username (`A-Z`, `a-z`, `0-9`, `_`) and press `Enter` or click **Use Offline** to apply immediately at runtime.
   - **Microsoft Genuine Sign-In**: Copy your Azure public-client Application ID and click **Set Azure Client ID** (or set `HORIZON_MSA_CLIENT_ID`), then click **Sign in with Microsoft (Device Code)**. The 8-character code is copied to your clipboard and your browser opens automatically; tokens are stored securely in **macOS Keychain** (`dev.horizon.macos.msa`).
   - **Replace Resources**: Click **Import / Replace Resources (.zip)…** at the bottom of the modal anytime to update your extracted game files.

---

## 3. Data & Save Locations on macOS

- **Game Directory (`saves/`, `options.txt`, `resourcepacks/`, `horizon-mods/`, `screenshots/`)**:
  `~/Library/Application Support/horizon/`
- **Extracted 26.3 Resources**:
  `~/Library/Application Support/horizon/resources/26.3/`
- **Account Store & Keychain Service**:
  `~/Library/Application Support/horizon/horizon_accounts.json` + macOS Keychain service `dev.horizon.macos.msa`.

---

## License & Attribution

- **Upstream Engine**: Built on [Horizon](https://github.com/horizon-rust/horizon), founded by [Ludovic Marie](https://lu4ovic.com) and developed by the Horizon Team and [contributors](https://github.com/horizon-rust/horizon/graphs/contributors).
- **License**: Original project code is licensed under the [GNU LGPL v3.0 or later](LICENSE); code merged prior to the LGPL transition remains available under the [MIT License](LICENSES/MIT.txt). Third-party notices are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

<sub>NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT. Minecraft is a trademark of Mojang Synergies AB; its code and assets belong to Mojang and Microsoft under the <a href="https://www.minecraft.net/en-us/eula">Minecraft EULA</a>.</sub>
