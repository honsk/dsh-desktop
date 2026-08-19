import { copyFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = join(__dirname, "src-tauri", "target", "release", "dsh-desktop.exe");
const outputDir = join(__dirname, "release", "portable");
const target = join(outputDir, "DSH Desktop.exe");

if (!existsSync(source)) {
  console.error(`未找到编译产物：${source}`);
  console.error("请先运行 npm run tauri build 或 npm run bundle:portable");
  process.exit(1);
}

mkdirSync(outputDir, { recursive: true });
copyFileSync(source, target);

console.log(`Portable 版已生成：${target}`);
