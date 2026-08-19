# DSH Desktop Shell

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
