# Horizon macOS 独立离线版（Apple Silicon M 芯片 + Intel 双架构）

本分支（`macos-offline`）将 `horizon-official`（`horizon-client`）打包为开箱即用的独立原生 **`Horizon.app`**，对齐 [horizon-android](https://github.com/samgum/horizon-android) 的免登录离线测试与一键资源包导入体验。

## 核心特性

1. **直接导入 `horizon-26.3-resources.zip` 即玩**
   - 首次打开 `Horizon.app` 时若未检测到已解压的游戏资源（默认存放于 `~/Library/Application Support/horizon/resources/26.3`），窗口会提供：
     - **一键导入 `Downloads/horizon-26.3-resources.zip`**（自动扫描 `~/Downloads` 下的资源包）
     - **选择资源压缩包 (`.zip`)…**（调起 macOS 原生文件选择框）
     - **拖拽导入**：直接将 `horizon-26.3-resources.zip` 拖入窗口即可实时显示解压进度，解压完成后自动启动进入主菜单（兼容顶层带 `26.3/` 目录或根目录直放 `horizon-resources.json` 两种压缩结构）。
   - 命令行同样支持直接传入 `.zip`：
     ```bash
     Horizon.app/Contents/MacOS/horizon-client --resources ~/Downloads/horizon-26.3-resources.zip
     ```

2. **免登录离线模式 + 微软正版双模式账号中心**
   - 默认零门槛启用 **离线模式 (`Player`)**，启动直达主菜单，可直接创建单人世界、局域网联机或连接离线服务器。
   - 点击主菜单左上角 **`离线: Player [账号]`** 胶囊即可打开内置账号与资源中心：
     - 随时自定义 `1~16` 位离线玩家名并实时生效；
     - 支持通过 **Microsoft OAuth 2.0 设备码** 登录正版账号（令牌通过 macOS 系统钥匙串 Keychain 安全存储并支持自动续期），也可一键粘贴自定义 Azure `Client ID`、导入调试 `Session JSON` 或更换 `.zip` 资源包。

3. **Apple Silicon (`arm64`) + Intel (`x86_64`) + Universal 全架构与 Mac 专属体验适配**
   - 修复了 `x86_64-apple-darwin` 在 Rosetta 2 及部分 macOS 环境下主线程 `GuestStackUnavailable` 退出问题。
   - 首次引导页内置 macOS 系统黑体（STHeiti / Hiragino）矢量中文字体回退，未导入资源包前中文提示清晰无乱码。
   - 原生适配 MacBook 触控板（Retina 双指平滑滚动、聊天记录逐行累计滚动、防惯性乱跳快捷栏切换、默认开启 `Control + 轻点` 模拟右键）与单声道近场脚步声/世界音效响度补偿。

## 本地编译与打包

```bash
# 1. 编译 Apple Silicon (M 系列芯片) 与 Intel 双架构 Release 产物
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin -p horizon-official --bin horizon-official
cargo build --release --locked --target x86_64-apple-darwin -p horizon-official --bin horizon-official

cp -f target/aarch64-apple-darwin/release/horizon-official target/aarch64-apple-darwin/release/horizon-client
cp -f target/x86_64-apple-darwin/release/horizon-official target/x86_64-apple-darwin/release/horizon-client

# 2. 生成 arm64 / x64 / universal 的 .app、.dmg 与 .zip 安装包
packaging/macos/package_game_app.sh target/aarch64-apple-darwin/release/horizon-client 26.3 dist arm64
packaging/macos/package_game_app.sh target/x86_64-apple-darwin/release/horizon-client 26.3 dist x64
packaging/macos/package_game_app.sh target/aarch64-apple-darwin/release/horizon-client 26.3 dist universal target/x86_64-apple-darwin/release/horizon-client
```

## 首次打开自签名版 `Horizon.app` 提示（Gatekeeper）

未配置 Apple Developer ID 公证时，应用使用 Ad-hoc 签名。若从浏览器下载后首次打开被系统拦截：
- **macOS 14 及更早版本**：在访达（Finder）中按住 Control 右键点击 `Horizon.app` → 选择「打开」。
- **macOS 15 及更新版本**：尝试打开一次后前往「系统设置 → 隐私与安全性」，点击底部的「仍要打开」，或执行 `xattr -cr /Applications/Horizon.app`。
