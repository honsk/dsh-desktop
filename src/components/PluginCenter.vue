<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface PluginConfig {
  id: string;
  name: string;
  description: string;
  commandTemplate: string;
  cwd: string | null;
  autoOpenUrl: string | null;
  icon: string;
  color: string;
  enabled: boolean;
}

interface PluginStatus {
  running: boolean;
  pid: number | null;
}

const plugins = ref<PluginConfig[]>([]);
const statuses = ref<Record<string, PluginStatus>>({});
const showEditor = ref(false);
const mode = ref<"form" | "json" | "url">("form");
const jsonText = ref("");
const importUrl = ref("");
const message = ref("");
const busy = ref(false);

const emptyPlugin = (): PluginConfig => ({
  id: "",
  name: "",
  description: "",
  commandTemplate: "npx @deepseek-ai/dsh web",
  cwd: null,
  autoOpenUrl: null,
  icon: "🧩",
  color: "#4F46E5",
  enabled: true,
});

const editing = reactive<PluginConfig>(emptyPlugin());
const cwdText = ref("");
const autoOpenUrlText = ref("");

function resetEditing() {
  Object.assign(editing, emptyPlugin());
  cwdText.value = "";
  autoOpenUrlText.value = "";
  jsonText.value = JSON.stringify(emptyPlugin(), null, 2);
}

async function loadPlugins() {
  try {
    plugins.value = await invoke<PluginConfig[]>("get_plugins");
  } catch (e) {
    message.value = String(e);
  }
}

async function loadStatuses() {
  try {
    statuses.value = await invoke<Record<string, PluginStatus>>("get_plugin_statuses");
  } catch (e) {
    message.value = String(e);
  }
}

function openCreate() {
  resetEditing();
  mode.value = "form";
  importUrl.value = "";
  showEditor.value = true;
}

function openEdit(plugin: PluginConfig) {
  Object.assign(editing, plugin);
  cwdText.value = plugin.cwd ?? "";
  autoOpenUrlText.value = plugin.autoOpenUrl ?? "";
  jsonText.value = JSON.stringify(plugin, null, 2);
  mode.value = "form";
  importUrl.value = "";
  showEditor.value = true;
}

function closeEditor() {
  showEditor.value = false;
}

function parseEditing(): PluginConfig {
  if (mode.value === "json") {
    const parsed = JSON.parse(jsonText.value) as PluginConfig;
    if (!parsed.id || !parsed.name) {
      throw new Error("插件 ID 和名称不能为空");
    }
    return parsed;
  }

  const data: PluginConfig = {
    id: editing.id.trim(),
    name: editing.name.trim(),
    description: editing.description.trim(),
    commandTemplate: editing.commandTemplate.trim(),
    cwd: cwdText.value.trim() || null,
    autoOpenUrl: autoOpenUrlText.value.trim() || null,
    icon: editing.icon || "🧩",
    color: editing.color || "#4F46E5",
    enabled: editing.enabled,
  };

  if (!data.id || !data.name || !data.commandTemplate) {
    throw new Error("ID、名称和启动命令不能为空");
  }

  return data;
}

async function savePlugin() {
  busy.value = true;
  message.value = "";
  try {
    const plugin = parseEditing();
    plugins.value = await invoke<PluginConfig[]>("save_plugin", { plugin });
    showEditor.value = false;
    message.value = "插件已保存";
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function importFromUrl() {
  const url = importUrl.value.trim();
  if (!url) {
    message.value = "请输入插件 JSON 地址";
    return;
  }

  busy.value = true;
  message.value = "";
  try {
    plugins.value = await invoke<PluginConfig[]>("import_plugin_from_url", { url });
    showEditor.value = false;
    message.value = "插件导入成功";
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}


async function deletePlugin(plugin: PluginConfig) {
  if (!confirm(`确定删除插件“${plugin.name}”吗？`)) return;
  busy.value = true;
  message.value = "";
  try {
    plugins.value = await invoke<PluginConfig[]>("delete_plugin", { id: plugin.id });
    delete statuses.value[plugin.id];
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function startPlugin(plugin: PluginConfig) {
  busy.value = true;
  message.value = "";
  try {
    const status = await invoke<PluginStatus>("start_plugin", { id: plugin.id });
    statuses.value[plugin.id] = status;
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function stopPlugin(plugin: PluginConfig) {
  busy.value = true;
  message.value = "";
  try {
    await invoke("stop_plugin", { id: plugin.id });
    statuses.value[plugin.id] = { running: false, pid: null };
  } catch (e) {
    message.value = String(e);
  } finally {
    busy.value = false;
  }
}

function statusOf(plugin: PluginConfig): PluginStatus {
  return statuses.value[plugin.id] || { running: false, pid: null };
}

onMounted(async () => {
  await Promise.all([loadPlugins(), loadStatuses()]);

  await listen<string>("plugin-status", (event) => {
    const id = event.payload;
    if (id) {
      loadStatuses();
    }
  });
});
</script>

<template>
  <div class="plugin-page">
    <div class="page-header">
      <div>
        <h1>插件中心</h1>
        <p class="hint">通过命令模板自由添加 DSH 模块入口，万物皆插件。</p>
      </div>
      <button class="primary" @click="openCreate">新增插件</button>
    </div>

    <div v-if="message" class="message">{{ message }}</div>

    <div v-if="plugins.length === 0" class="empty">
      还没有插件，点击“新增插件”创建第一个入口。
    </div>

    <div v-else class="plugin-grid">
      <div v-for="plugin in plugins" :key="plugin.id" class="plugin-card">
        <div class="plugin-icon" :style="{ background: plugin.color + '22', color: plugin.color }">
          {{ plugin.icon }}
        </div>
        <div class="plugin-info">
          <div class="plugin-name">
            {{ plugin.name }}
            <span class="status-badge" :class="statusOf(plugin).running ? 'badge-running' : 'badge-stopped'">
              {{ statusOf(plugin).running ? "运行中" : "未运行" }}
            </span>
          </div>
          <div class="plugin-desc">{{ plugin.description || "暂无描述" }}</div>
          <code class="plugin-command">{{ plugin.commandTemplate }}</code>
          <div v-if="statusOf(plugin).pid" class="plugin-pid">PID: {{ statusOf(plugin).pid }}</div>
        </div>
        <div class="plugin-actions">
          <button
            class="primary"
            :disabled="busy || statusOf(plugin).running || !plugin.enabled"
            @click="startPlugin(plugin)"
          >
            启动
          </button>
          <button :disabled="busy || !statusOf(plugin).running" @click="stopPlugin(plugin)">
            停止
          </button>
          <button :disabled="busy" @click="openEdit(plugin)">编辑</button>
          <button class="danger" :disabled="busy" @click="deletePlugin(plugin)">删除</button>
        </div>
      </div>
    </div>

    <div v-if="showEditor" class="modal-mask" @click.self="closeEditor">
      <div class="modal">
        <div class="modal-header">
          <h2>{{ editing.id ? "编辑插件" : "新增插件" }}</h2>
          <button @click="closeEditor">✕</button>
        </div>

        <div class="modal-tabs">
          <button :class="{ active: mode === 'form' }" @click="mode = 'form'">表单</button>
          <button :class="{ active: mode === 'json' }" @click="mode = 'json'">JSON</button>
            <button :class="{ active: mode === 'url' }" @click="mode = 'url'">链接安装</button>
        </div>

        <div v-if="mode === 'form'" class="modal-body">
          <div class="form-item">
            <label>ID</label>
            <input v-model="editing.id" type="text" placeholder="例如 dsh-web" />
          </div>
          <div class="form-item">
            <label>名称</label>
            <input v-model="editing.name" type="text" placeholder="例如 DSH Web" />
          </div>
          <div class="form-item">
            <label>描述</label>
            <input v-model="editing.description" type="text" placeholder="这个插件做什么" />
          </div>
          <div class="form-item">
            <label>启动命令模板</label>
            <input v-model="editing.commandTemplate" type="text" placeholder="npx @deepseek-ai/dsh web" />
            <span class="field-hint">可用变量：{port}</span>
          </div>
          <div class="form-item">
            <label>工作目录（可选）</label>
            <input v-model="cwdText" type="text" placeholder="留空则使用当前目录" />
          </div>
          <div class="form-item">
            <label>自动打开地址（可选）</label>
            <input v-model="autoOpenUrlText" type="text" placeholder="http://127.0.0.1:{port}" />
          </div>
          <div class="form-row">
            <div class="form-item">
              <label>图标</label>
              <input v-model="editing.icon" type="text" placeholder="🧩" />
            </div>
            <div class="form-item">
              <label>颜色</label>
              <input v-model="editing.color" type="color" />
            </div>
          </div>
          <div class="form-item checkbox">
            <label>
              <input v-model="editing.enabled" type="checkbox" />
              启用该插件
            </label>
          </div>
        </div>

        <div v-else-if="mode === 'json'" class="modal-body">
          <textarea v-model="jsonText" class="json-editor" rows="18" spellcheck="false"></textarea>
        </div>

          <div v-else class="modal-body">
            <div class="form-item">
              <label>插件 JSON 地址</label>
              <input v-model="importUrl" type="text" placeholder="支持 URL、GitHub 仓库、本地文件路径" />
              <span class="field-hint">支持单个插件 JSON、插件数组 JSON、或 { "plugins": [...] } 格式；GitHub 会自动尝试 raw / jsDelivr / 代理加速</span>
                <span class="field-hint">如果 GitHub 导入失败，可以手动下载 JSON 后使用本地文件路径导入</span>

            </div>
            <div class="actions">
              <button class="primary" :disabled="busy" @click="importFromUrl">获取并安装</button>
            </div>
          </div>


        <div class="modal-footer">
          <button @click="closeEditor">取消</button>
          <button v-if="mode !== 'url'" class="primary" :disabled="busy" @click="savePlugin">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plugin-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.hint {
  color: #57606a;
  margin: 4px 0 0;
}

.message {
  background: #fff8c5;
  border: 1px solid #d4a72c;
  color: #7a5900;
  padding: 10px 14px;
  border-radius: 8px;
}

.empty {
  background: #ffffff;
  border: 1px dashed #d0d7de;
  border-radius: 12px;
  padding: 40px;
  text-align: center;
  color: #57606a;
}

.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 14px;
}

.plugin-card {
  background: #ffffff;
  border: 1px solid #d0d7de;
  border-radius: 12px;
  padding: 16px;
  display: flex;
  gap: 14px;
}

.plugin-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
}

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-name {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.status-badge {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 500;
}

.badge-running {
  background: #dafbe1;
  color: #1a7f37;
}

.badge-stopped {
  background: #eff2f5;
  color: #57606a;
}

.plugin-desc {
  color: #57606a;
  font-size: 13px;
  margin-top: 2px;
}

.plugin-command {
  display: block;
  background: #f6f8fa;
  border-radius: 6px;
  padding: 6px 8px;
  margin-top: 8px;
  font-size: 12px;
  font-family: Consolas, "Courier New", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-pid {
  font-size: 12px;
  color: #57606a;
  margin-top: 4px;
}

.plugin-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  justify-content: center;
}

.plugin-actions button {
  white-space: nowrap;
  padding: 6px 12px;
  font-size: 13px;
}

button.danger {
  border-color: #e5534b;
  color: #e5534b;
}

button.danger:hover {
  background: #e5534b;
  color: #ffffff;
}

.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  background: #ffffff;
  border-radius: 14px;
  width: 640px;
  max-width: 92vw;
  max-height: 88vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.2);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #d0d7de;
}

.modal-header h2 {
  margin: 0;
  font-size: 18px;
}

.modal-tabs {
  display: flex;
  gap: 8px;
  padding: 12px 20px 0;
}

.modal-tabs button.active {
  background: #4f46e5;
  color: #ffffff;
  border-color: #4f46e5;
}

.modal-body {
  padding: 16px 20px;
  overflow: auto;
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.form-item {
  margin-bottom: 14px;
}

.form-item label {
  display: block;
  font-weight: 500;
  margin-bottom: 6px;
}

.form-item input,
.form-item textarea {
  width: 100%;
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

.json-editor {
  font-family: Consolas, "Courier New", monospace;
  font-size: 13px;
  border: 1px solid #d0d7de;
  border-radius: 8px;
  padding: 10px;
  resize: vertical;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 20px;
  border-top: 1px solid #d0d7de;
}
</style>
