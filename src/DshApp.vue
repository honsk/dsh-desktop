<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import PluginCenter from "./components/PluginCenter.vue";

interface Settings {
  dshCommandTemplate: string;
  port: number;
  autoOpenBrowser: boolean;
  autoStartOnLaunch: boolean;
  autoLaunchOnBoot: boolean;
  minimizeToTray: boolean;
  globalShortcut: string;
  theme: string;
  logRetentionDays: number;
}

interface DshStatus {
  running: boolean;
  pid: number | null;
  state: string;
  exitCode: number | null;
}

const currentTab = ref<"launch" | "plugins" | "settings" | "logs">("launch");
const status = ref<DshStatus>({
  running: false,
  pid: null,
  state: "stopped",
  exitCode: null,
});
const settings = ref<Settings>({
  dshCommandTemplate: "npx @deepseek-ai/dsh web",
  port: 3080,
  autoOpenBrowser: true,
  autoStartOnLaunch: false,
  autoLaunchOnBoot: false,
  minimizeToTray: true,
  globalShortcut: "Alt+Shift+D",
  theme: "system",
  logRetentionDays: 7,
});
const logs = ref("");
const message = ref("");
const busy = ref(false);

async function refreshStatus() {
  try {
    status.value = await invoke<DshStatus>("get_dsh_status");
  } catch (e) {
    message.value = String(e);
  }
}

async function refreshLogs() {
  try {
    logs.value = await invoke<string>("get_logs");
  } catch (e) {
    message.value = String(e);
  }
}

async function loadSettings() {
  try {
    settings.value = await invoke<Settings>("get_settings");
  } catch (e) {
    message.value = String(e);
  }
}

async function startDsh() {
  busy.value = true;
  message.value = "";
  try {
    status.value = await invoke<DshStatus>("start_dsh");
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function stopDsh() {
  busy.value = true;
  message.value = "";
  try {
    await invoke("stop_dsh");
    await refreshStatus();
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function openLauncher() {
  const launcher = await WebviewWindow.getByLabel("launcher");
  if (launcher) {
    await launcher.show();
    await launcher.setFocus();
  }
}

async function openWeb() {
  try {
    await invoke("open_dsh_web");
  } catch (e) {
    message.value = String(e);
  }
}

async function saveSettings() {
  busy.value = true;
  message.value = "";
  try {
    await invoke("save_settings", { settings: settings.value });
    message.value = status.value.running
        ? "设置已保存，部分修改需要停止并重新启动 DSH 后生效"
        : "设置已保存";
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function clearLogs() {
  try {
    await invoke("clear_logs");
    logs.value = "";
  } catch (e) {
    message.value = String(e);
  }
}

function statusText() {
  switch (status.value.state) {
    case "running":
      return "运行中";
    case "starting":
      return "启动中";
    case "exited":
      return "已退出";
    case "error":
      return "异常";
    default:
      return "未启动";
  }
}

function statusClass() {
  return status.value.running ? "status-running" : "status-stopped";
}

const themeClass = computed(() => {
  if (settings.value.theme === "dark") return "theme-dark";
  if (settings.value.theme === "light") return "theme-light";
  return "theme-system";
});

onMounted(async () => {
  await Promise.all([loadSettings(), refreshStatus(), refreshLogs()]);

  await listen<string>("dsh-log", (event) => {
    logs.value += event.payload + "\n";
  });
  await listen<string>("dsh-status", () => {
    refreshStatus();
  });
});
</script>

<template>
  <div class="app-shell" :class="themeClass">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-icon">⚡</span>
        <span>DSH Desktop</span>
      </div>
      <nav>
        <button
          :class="{ active: currentTab === 'launch' }"
          @click="currentTab = 'launch'"
        >
          🚀 快速启动
        </button>
          <button
            :class="{ active: currentTab === 'plugins' }"
            @click="currentTab = 'plugins'"
          >
            🧩 插件中心
          </button>

        <button
          :class="{ active: currentTab === 'settings' }"
          @click="currentTab = 'settings'"
        >
          ⚙️ 设置
        </button>
        <button
          :class="{ active: currentTab === 'logs' }"
          @click="currentTab = 'logs'"
        >
          📄 日志
        </button>
      </nav>
      <div class="sidebar-footer">v0.2.0 · M2</div>
    </aside>

    <main class="content">
      <div v-if="message" class="message">{{ message }}</div>

      <section v-if="currentTab === 'launch'" class="page">
        <h1>快速启动台</h1>
        <p class="hint">一键启动 DeepSeek Harness Web，并自动打开浏览器。</p>

        <div class="status-card" :class="statusClass()">
          <div class="status-dot" />
          <div>
            <div class="status-title">{{ statusText() }}</div>
            <div class="status-meta">
              <span v-if="status.pid">PID: {{ status.pid }}</span>
              <span v-else>尚未启动</span>
              <span v-if="status.exitCode !== null">
                退出码: {{ status.exitCode }}
              </span>
            </div>
          </div>
        </div>

        <div class="actions">
          <button class="primary" :disabled="busy || status.running" @click="startDsh">
            {{ status.running ? "已在运行" : "启动 DSH Web" }}
          </button>
          <button :disabled="busy || !status.running" @click="openWeb">
            打开网页
          </button>
          <button :disabled="busy || !status.running" @click="stopDsh">
            停止
          </button>
            <button @click="openLauncher">启动器</button>

        </div>

        <div class="info-box">
          <strong>启动命令</strong>
          <code>{{ settings.dshCommandTemplate }}</code>
          <div class="info-meta">
            默认地址：http://127.0.0.1:{{ settings.port }}
          </div>
        </div>
      </section>

        <section v-else-if="currentTab === 'plugins'" class="page">
          <PluginCenter />
        </section>


      <section v-else-if="currentTab === 'settings'" class="page">
        <h1>设置</h1>

        <div class="form-item">
          <label>DSH 启动命令模板</label>
          <input v-model="settings.dshCommandTemplate" type="text" />
          <span class="field-hint">可用变量：{port} · 下次启动 DSH 时生效</span>
        </div>

        <div class="form-item">
          <label>端口</label>
          <input v-model.number="settings.port" type="number" min="1" max="65535" />
            <span class="field-hint">仅在命令模板包含 {port} 时生效</span>
        </div>

        <div class="form-item">
          <label>主题</label>
          <select v-model="settings.theme">
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </div>

        <div class="form-item">
          <label>全局快捷键</label>
          <input v-model="settings.globalShortcut" type="text" disabled />
            <span class="field-hint badge-pending">开发中，暂未生效</span>
        </div>

        <div class="form-item">
          <label>日志保留天数</label>
          <input v-model.number="settings.logRetentionDays" type="number" min="1" />
            <span class="field-hint">启动时自动清理旧日志</span>
        </div>

        <div class="form-item checkbox">
          <label>
            <input v-model="settings.autoOpenBrowser" type="checkbox" />
            启动后自动打开浏览器
          </label>
        </div>

        <div class="form-item checkbox">
          <label>
            <input v-model="settings.autoStartOnLaunch" type="checkbox" />
            桌面版启动时自动启动 DSH Web
          </label>
        </div>

          <div class="form-item checkbox">
            <label>
              <input v-model="settings.autoLaunchOnBoot" type="checkbox" />
              开机时自动启动 DSH Desktop
            </label>
          </div>


        <div class="form-item checkbox">
          <label>
            <input v-model="settings.minimizeToTray" type="checkbox" />
            关闭窗口时最小化到托盘
          </label>
        </div>

        <div class="actions">
          <button class="primary" :disabled="busy" @click="saveSettings">
            保存设置
          </button>
        </div>
      </section>

      <section v-else class="page">
        <div class="page-header">
          <h1>日志</h1>
          <div>
            <button @click="refreshLogs">刷新</button>
            <button @click="clearLogs">清空</button>
          </div>
        </div>
        <pre class="log-view">{{ logs || "暂无日志" }}</pre>
      </section>
    </main>
  </div>
</template>

<style>
:root {
  font-family: "Segoe UI", "Microsoft YaHei", Inter, Avenir, Helvetica, Arial,
    sans-serif;
  font-size: 15px;
  line-height: 1.6;
  color: #1f2328;
  background-color: #f5f6f8;
}

* {
  box-sizing: border-box;
}

html,
body {
  height: 100%;
  margin: 0;
  overflow: hidden;
}

button,
input,
select {
  font: inherit;
}

button {
  border: 1px solid #d0d7de;
  background: #ffffff;
  border-radius: 8px;
  padding: 8px 16px;
  cursor: pointer;
  transition: all 0.15s ease;
}

button:hover:not(:disabled) {
  border-color: #4f46e5;
  color: #4f46e5;
}

button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

button.primary {
  background: #4f46e5;
  border-color: #4f46e5;
  color: #ffffff;
}

button.primary:hover:not(:disabled) {
  background: #4338ca;
  color: #ffffff;
}

input,
select {
  border: 1px solid #d0d7de;
  border-radius: 8px;
  padding: 8px 12px;
  background: #ffffff;
  width: 100%;
}

.app-shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.sidebar {
  width: 220px;
  background: #1f2328;
  color: #e6e6e6;
  display: flex;
  flex-direction: column;
  padding: 16px 12px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
  padding: 8px 12px 20px;
}

.brand-icon {
  font-size: 22px;
}

.sidebar nav {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sidebar nav button {
  background: transparent;
  border: none;
  color: #c9d1d9;
  text-align: left;
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 14px;
}

.sidebar nav button:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
}

.sidebar nav button.active {
  background: #4f46e5;
  color: #ffffff;
}

.sidebar-footer {
  margin-top: auto;
  font-size: 12px;
  color: #8b949e;
  padding: 8px 12px;
}

.content {
  flex: 1;
  min-height: 0;
  height: 100vh;
  padding: 32px;
  overflow-y: auto;
  overflow-x: hidden;
}

.page h1 {
  margin-top: 0;
  margin-bottom: 8px;
  font-size: 24px;
}

.hint {
  color: #57606a;
  margin-top: 0;
}

.message {
  background: #fff8c5;
  border: 1px solid #d4a72c;
  color: #7a5900;
  padding: 10px 14px;
  border-radius: 8px;
  margin-bottom: 16px;
}

.status-card {
  display: flex;
  align-items: center;
  gap: 14px;
  background: #ffffff;
  border: 1px solid #d0d7de;
  border-radius: 12px;
  padding: 16px 20px;
  margin: 20px 0;
}

.status-card.status-running {
  border-color: #2da44e;
}

.status-card.status-stopped {
  border-color: #d0d7de;
}

.status-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #d0d7de;
}

.status-running .status-dot {
  background: #2da44e;
  box-shadow: 0 0 0 4px rgba(45, 164, 78, 0.15);
}

.status-title {
  font-weight: 600;
  font-size: 16px;
}

.status-meta {
  color: #57606a;
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 10px;
  margin: 16px 0;
  flex-wrap: wrap;
}

.info-box {
  background: #ffffff;
  border: 1px solid #d0d7de;
  border-radius: 12px;
  padding: 16px 20px;
  margin-top: 16px;
}

.info-box code {
  display: block;
  background: #f6f8fa;
  border-radius: 6px;
  padding: 8px 10px;
  margin-top: 8px;
  font-family: Consolas, "Courier New", monospace;
}

.info-meta {
  color: #57606a;
  font-size: 13px;
  margin-top: 8px;
}

.form-item {
  margin-bottom: 18px;
}

.form-item label {
  display: block;
  font-weight: 500;
  margin-bottom: 6px;
}

.form-item.checkbox label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 400;
}

.form-item.checkbox input {
  width: auto;
}

.field-hint {
  font-size: 12px;
  color: #57606a;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.log-view {
  background: #0d1117;
  color: #c9d1d9;
  border-radius: 12px;
  padding: 16px;
  min-height: 400px;
  max-height: 70vh;
  overflow: auto;
  font-family: Consolas, "Courier New", monospace;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}

.badge-pending {
  color: #9a6700;
  font-weight: 500;
}

.theme-dark .content {
  background: #1f2328;
  color: #e6e6e6;
}

.theme-dark .status-card,
.theme-dark .info-box {
  background: #2d333b;
  border-color: #444c56;
  color: #e6e6e6;
}

.theme-dark .status-meta,
.theme-dark .info-meta,
.theme-dark .field-hint {
  color: #8b949e;
}

.theme-dark input,
.theme-dark select {
  background: #0d1117;
  border-color: #444c56;
  color: #e6e6e6;
}

.theme-dark .message {
  background: #3a2d00;
  border-color: #9a6700;
  color: #f0d28c;
}

</style>
