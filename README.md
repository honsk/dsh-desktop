# DSH Desktop

> Windows 个人使用的 DSH（DeepSeek-Harness）快速启动桌面壳。  
> 轻量、快捷、插件化，与网页版 DSH 分离。

基于 **Tauri 2 + Vue 3 + TypeScript + Rust** 构建。

---

## ✨ 功能特性

- 🚀 一键启动 DSH Web
- 🌐 自动打开 `http://127.0.0.1:3080`
- 🧩 插件中心：新增、编辑、删除、启动、停止插件
- ⚡ 轻量启动器：全局快捷键 `Alt + Shift + D`
- 🖥️ 系统托盘：快速启动 / 停止 / 打开网页
- ⚙️ 设置管理：命令模板、端口、主题、日志保留天数
- 🔐 开机自启动（可选）
- 📄 日志查看与清理
- 📦 Portable 版 + NSIS 安装版

---

## 🖼️ 截图

> 待补充：可以在这里放主界面和启动器的截图。

---

## 📁 项目结构

```text
dsh-desktop/
├── src/                 # Vue 前端
│   ├── DshApp.vue       # 主界面
│   ├── Launcher.vue     # 轻量启动器
│   └── components/      # 组件
├── src-tauri/           # Tauri / Rust 后端
│   ├── src/
│   │   └── lib.rs       # 核心逻辑
│   └── tauri.conf.json  # Tauri 配置
├── copy-portable.mjs    # Portable 版复制脚本
└── package.json
```

---

## 🚀 本地开发

### 环境要求

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/)
- Windows 需要 [Visual Studio Build Tools](https://visualstudio.microsoft.com/)

### 启动开发模式

```powershell
cd 你的项目目录
npm install
npm run tauri dev
```

---

## 📦 打包

### 同时生成安装版 + Portable 版

```powershell
npm run bundle
```

### 只生成 Portable 版

```powershell
npm run bundle:portable
```

### 只生成安装版

```powershell
npm run bundle:installer
```

### 产物位置

| 类型 | 路径 |
|---|---|
| 安装版 | `src-tauri/target/release/bundle/nsis/` |
| Portable 版 | `release/portable/DSH Desktop.exe` |

---

## ⌨️ 快捷键

| 快捷键 | 功能 |
|---|---|
| `Alt + Shift + D` | 打开 / 关闭轻量启动器 |
| `Esc` | 关闭启动器 |

---

## 🔒 安全说明

- 本项目的用户数据保存在系统用户目录下，不会上传到仓库
- 请勿将个人 Token、API Key、密码等敏感信息写入源码
- 如果涉及密钥，建议使用环境变量或本地配置文件

---

## 🧩 插件

插件通过 JSON 配置管理，默认包含：

```json
{
  "id": "dsh-web",
  "name": "DSH Web",
  "description": "启动 DeepSeek Harness 网页版",
  "commandTemplate": "npx @deepseek-ai/dsh web",
  "autoOpenUrl": "http://127.0.0.1:{port}",
  "icon": "🌐",
  "enabled": true
}
```

支持：

- 表单编辑
- JSON 编辑
- 从链接导入
- 本地文件导入
- GitHub 仓库导入（多镜像加速）

---

## 📝 License

> 待补充：你可以选择 MIT、Apache 2.0 等开源协议。

<!--

Windows 个人使用的 DSH 快速启动桌面壳，基于 Tauri 2 + Vue 3 + TypeScript。

## 当前进度（M4）

- [x] 一键启动 `npx @deepseek-ai/dsh web`
- [x] 自动打开 `http://127.0.0.1:3080`
- [x] 启动 / 停止 / 打开网页
- [x] 基础日志查看与清空
- [x] 设置页（命令模板、端口、自动打开浏览器等）
- [x] 系统托盘菜单
- [x] 插件中心（M2：JSON 配置 + 增删改查 + 启动）
- [x] 轻量启动器弹窗（M3）
- [x] Portable + 安装包（M4）

## 本地开发

```powershell
cd 你的项目目录
npm install
npm run tauri dev
```

> 如果提示缺少 Rust，请先安装：https://www.rust-lang.org/learn/get-started

## 打包

### 同时生成安装版 + Portable 版

```powershell
npm run bundle
```

### 只生成 Portable 版

```powershell
npm run bundle:portable
```

### 只生成安装版

```powershell
npm run bundle:installer
```

产物位置：

- 安装版：`src-tauri/target/release/bundle/nsis/`
- Portable 版：`release/portable/DSH Desktop.exe`

<!--

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
-->
