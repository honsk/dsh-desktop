import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import DshApp from "./DshApp.vue";
import Launcher from "./Launcher.vue";

const currentWindow = getCurrentWindow();
const rootComponent = currentWindow.label === "launcher" ? Launcher : DshApp;

createApp(rootComponent).mount("#app");
