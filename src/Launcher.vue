<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

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

const plugins = ref<PluginConfig[]>([]);
const keyword = ref("");
const selectedIndex = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);
const appWindow = getCurrentWindow();

const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase();
  if (!kw) return plugins.value;
  return plugins.value.filter((p) => {
    return (
      p.name.toLowerCase().includes(kw) ||
      p.id.toLowerCase().includes(kw) ||
      p.description.toLowerCase().includes(kw) ||
      p.commandTemplate.toLowerCase().includes(kw)
    );
  });
});

async function loadPlugins() {
  try {
    plugins.value = await invoke<PluginConfig[]>("get_plugins");
  } catch (e) {
    console.error(e);
  }
}

async function hideLauncher() {
  try {
    await appWindow.hide();
  } catch (e) {
    console.error("隐藏启动器失败", e);
  }
}

async function launchSelected() {
  const plugin = filtered.value[selectedIndex.value];
  if (!plugin) return;
  try {
    await invoke("start_plugin", { id: plugin.id });
  } catch (e) {
    console.error(e);
  }
  await hideLauncher();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    hideLauncher();
    return;
  }

  if (event.key === "ArrowDown") {
    event.preventDefault();
    if (filtered.value.length > 0) {
      selectedIndex.value = (selectedIndex.value + 1) % filtered.value.length;
    }
    return;
  }

  if (event.key === "ArrowUp") {
    event.preventDefault();
    if (filtered.value.length > 0) {
      selectedIndex.value =
        (selectedIndex.value - 1 + filtered.value.length) % filtered.value.length;
    }
    return;
  }

  if (event.key === "Enter") {
    event.preventDefault();
    launchSelected();
  }
}

async function focusInput() {
  await nextTick();
  inputRef.value?.focus();
  inputRef.value?.select();
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    hideLauncher();
  }
}

onMounted(async () => {
  await loadPlugins();
  await focusInput();

  window.addEventListener("keydown", onGlobalKeydown);

  await listen<string>("launcher-show", () => {
    keyword.value = "";
    selectedIndex.value = 0;
    loadPlugins();
    focusInput();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});
</script>

<template>
  <div class="launcher">
    <input
      ref="inputRef"
      v-model="keyword"
      class="launcher-input"
      type="text"
      placeholder="搜索并启动插件..."
      @keydown="onKeydown"
    />
      <button class="close-btn" @click="hideLauncher">✕</button>

    <div v-if="filtered.length > 0" class="launcher-list">
      <div
        v-for="(plugin, index) in filtered"
        :key="plugin.id"
        class="launcher-item"
        :class="{ active: index === selectedIndex }"
        @mousedown.prevent="selectedIndex = index; launchSelected()"
      >
        <span class="item-icon">{{ plugin.icon }}</span>
        <div class="item-info">
          <div class="item-name">{{ plugin.name }}</div>
          <div class="item-command">{{ plugin.commandTemplate }}</div>
        </div>
      </div>
    </div>
    <div v-else class="launcher-empty">没有匹配的插件</div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  font-family: "Segoe UI", "Microsoft YaHei", sans-serif;
}

.launcher {
  position: relative;
  width: 100%;
  height: 100%;
  background: #1e1e1e;
  border-radius: 12px;
  padding: 12px;
  color: #e6e6e6;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.launcher-input {
  width: 100%;
  border: none;
  outline: none;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 16px;
  color: #ffffff;
}

.close-btn {
  position: absolute;
  top: 14px;
  right: 14px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  color: #ffffff;
  width: 28px;
  height: 28px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.2);
}


.launcher-list {
  margin-top: 8px;
  max-height: 320px;
  overflow: auto;
}

.launcher-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
}

.launcher-item.active {
  background: rgba(79, 70, 229, 0.7);
}

.item-icon {
  font-size: 20px;
}

.item-info {
  min-width: 0;
}

.item-name {
  font-weight: 600;
}

.item-command {
  font-size: 12px;
  color: #a0a0a0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.launcher-empty {
  padding: 12px;
  color: #a0a0a0;
  text-align: center;
}
</style>
