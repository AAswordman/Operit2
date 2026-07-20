import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryDirectory = path.resolve(scriptDirectory, "../..");
const v86BuildDirectory = path.join(
  repositoryDirectory,
  "apps",
  "flutter",
  "app",
  ".dart_tool",
  "web-build-deps",
  "node_modules",
  "v86",
  "build",
);
const v86AssetDirectory = path.join(
  repositoryDirectory,
  "apps",
  "web_access",
  "web",
  "v86",
);
const runtimeDirectory = path.join(
  v86AssetDirectory,
  "runtime",
);

/** Returns the absolute path for one V86 npm package build artifact. */
function v86BuildPath(name) {
  return path.join(v86BuildDirectory, name);
}

/** Returns the absolute path for one BIOS artifact staged for the Web host. */
function v86AssetPath(name) {
  return path.join(v86AssetDirectory, name);
}

/** Returns the absolute path for one generated V86 runtime artifact. */
function runtimePath(name) {
  return path.join(runtimeDirectory, name);
}

/** Boots the suite and verifies interactive Node plus both Python command names over serial. */
async function main() {
  const { V86 } = await import(pathToFileURL(v86BuildPath("libv86.mjs")).href);
  const encoder = new TextEncoder();
  let serialOutput = "";
  let phase = "waiting";
  let pythonFailureObserved = false;
  const emulator = new V86({
    wasm_path: v86BuildPath("v86.wasm"),
    memory_size: 512 * 1024 * 1024,
    vga_memory_size: 2 * 1024 * 1024,
    bios: { url: v86AssetPath("seabios.bin") },
    vga_bios: { url: v86AssetPath("vgabios.bin") },
    bzimage: { url: runtimePath("operit-runtime-bzimage.bin") },
    initrd: { url: runtimePath("operit-runtime-initrd.cpio.gz") },
    cmdline: "console=ttyS0 operit.mode=terminal operit.rows=32 operit.cols=120 tsc=reliable mitigations=off random.trust_cpu=on",
    autostart: true,
  });

  /** Sends one complete command sequence after V86 finishes its current serial callback. */
  function sendGuestInput(bytes) {
    setTimeout(() => {
      emulator.serial_send_bytes(0, encoder.encode(bytes));
    }, 100);
  }

  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`V86 guest did not start Node and Python:\n${serialOutput}`));
    }, 90000);
    emulator.add_listener("serial0-output-byte", value => {
      if (typeof value !== "number") {
        return;
      }
      serialOutput += String.fromCharCode(value & 0xff);
      if (!pythonFailureObserved && serialOutput.includes("Fatal Python error: init_fs_encoding")) {
        pythonFailureObserved = true;
        clearTimeout(timeout);
        setTimeout(() => {
          reject(new Error(`V86 guest Python initialization failed:\n${serialOutput}`));
        }, 1000);
        return;
      }
      if (phase === "waiting" && serialOutput.includes("OPERIT_TERMINAL_READY")) {
        phase = "starting-node";
        sendGuestInput("stty size\nnode\n");
      } else if (phase === "starting-node" && serialOutput.includes("Welcome to Node.js v20.19.0.")) {
        phase = "evaluating-node";
        sendGuestInput("100000 + 22222\n");
      } else if (phase === "evaluating-node" && serialOutput.includes("122222")) {
        phase = "leaving-node";
        sendGuestInput(".exit\n");
      } else if (phase === "leaving-node" && serialOutput.endsWith("/ # ")) {
        phase = "starting-python";
        sendGuestInput("python\n");
      } else if (phase === "starting-python" && serialOutput.endsWith(">>> ")) {
        phase = "running-python";
        sendGuestInput("print(41 + 1)\n");
      } else if (phase === "running-python" && serialOutput.includes("42")) {
        phase = "leaving-python";
        sendGuestInput("exit()\n");
      } else if (phase === "leaving-python" && serialOutput.endsWith("/ # ")) {
        phase = "checking-python-version";
        sendGuestInput("python --version\n");
      } else if (
        phase === "checking-python-version" &&
        serialOutput.includes("Python 3.12.11")
      ) {
        phase = "checking-python3-version";
        sendGuestInput("python3 --version\n");
      } else if (
        phase === "checking-python3-version" &&
        Array.from(serialOutput.matchAll(/Python 3\.12\.11/g)).length >= 3
      ) {
        phase = "checking-node-builtins";
        sendGuestInput("node -e \"const dayjs=require('dayjs'); const _=require('lodash'); const {z}=require('zod'); const uuid=require('uuid'); console.log('node-builtins:'+dayjs('2026-07-20').format('YYYY')+':'+_.chunk([1,2],1).length+':'+z.string().parse('ok')+':'+(typeof uuid.v4))\"\n");
      } else if (phase === "checking-node-builtins" && serialOutput.includes("node-builtins:2026:2:ok:function")) {
        phase = "checking-python-builtins";
        sendGuestInput("python -c \"import click, dateutil, requests; from packaging.version import Version; from rich.console import Console; print('python-builtins:'+requests.__version__+':'+dateutil.__version__+':'+str(Version('1.2.3'))+':'+click.__version__)\"\n");
      } else if (phase === "checking-python-builtins" && serialOutput.includes("python-builtins:2.32.3:2.9.0.post0:1.2.3:8.1.8")) {
        phase = "checking-python-pip";
        sendGuestInput("python -m pip --version\n");
      } else if (phase === "checking-python-pip" && serialOutput.includes("No module named pip")) {
        phase = "checking-package-managers";
        sendGuestInput("test ! -e /usr/local/bin/npm && test ! -e /usr/local/bin/npx && test ! -e /usr/local/bin/pip && test ! -e /usr/local/bin/pip3 && echo package-managers-absent\n");
      }
      if (
        serialOutput.includes("OPERIT_TERMINAL_READY") &&
        serialOutput.includes("Welcome to Node.js v20.19.0.") &&
        serialOutput.includes("32 120") &&
        serialOutput.includes("122222") &&
        serialOutput.includes("42") &&
        serialOutput.includes("node-builtins:2026:2:ok:function") &&
        serialOutput.includes("python-builtins:2.32.3:2.9.0.post0:1.2.3:8.1.8") &&
        serialOutput.includes("No module named pip") &&
        serialOutput.includes("package-managers-absent")
      ) {
        clearTimeout(timeout);
        resolve();
      }
    });
  });
  await emulator.destroy();
  process.stdout.write(serialOutput);
}

await main();
