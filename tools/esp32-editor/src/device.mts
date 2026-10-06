import {readFile, writeFile} from 'node:fs/promises';
import {createInterface} from 'node:readline/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {editorRoot, configFile, firmwareElf, loadDeviceConfig, listSerialPorts, selectSerialPort,
  buildFlashPlan, executeFlashPlan, runTool} from './device-tools.mts';

/** Small npm entrypoint: no HTTP server required for build/flash/monitor. */
export async function deviceMain(args: string[]): Promise<void> {
  const action = args[0] ?? 'help';
  const flags = args.slice(1);
  const valueFlags = new Set(['--port', '--emsdk', '--idf', '--id', '--direction', '--timeout', '--bridge', '--text']);
  const booleanFlags = new Set(['--reset-data', '--confirm-reset', '--no-monitor', '--dry-run', '--help']);
  const values = new Map<string, string>();
  const enabled = new Set<string>();
  for (let i = 0; i < flags.length; i++) {
    const flag = flags[i]!;
    if (valueFlags.has(flag)) {
      const value = flags[++i];
      if (!value || value.startsWith('--')) throw new Error(`${flag} 需要参数`);
      values.set(flag, value);
    } else if (booleanFlags.has(flag)) enabled.add(flag);
    else throw new Error(`未知参数 ${flag}`);
  }
  if (action === 'help' || enabled.has('--help')) {
    console.log('npm run dev        构建 → 更新固件 → 串口日志\nnpm run flash      更新已有固件，保留 Wi-Fi/配对/空间/布局\nnpm run flash:fresh 清空设备后安装（需确认，配对身份也会删除）\nnpm run dev:fresh   重新构建 → 清空安装 → 串口日志（需确认）\nnpm run monitor    串口日志（不主动重启）\nnpm run ports      查看串口\nnpm run device:setup -- --emsdk <路径> [--port COMxx]\n真机读屏：npm run device:screen -- --port COMxx\n真机对象树：npm run device:tree -- --port COMxx\n真机内存：npm run device:health -- --port COMxx\n真机点击：npm run device:tap -- --port COMxx --id <屏幕节点ID>\n自绘 UI：npm run dev（唯一渲染后端）\nSDK 配置：npm run device:setup -- --idf <ESP-IDF目录>\n追加参数：--port COMxx / --dry-run（仅 flash）/ --no-monitor（仅 dev）');
    return;
  }
  if (!['setup', 'ports', 'flash', 'monitor', 'dev', 'screen', 'tree', 'tap', 'swipe', 'health', 'draft'].includes(action)) throw new Error(`未知命令 ${action}`);
  if (!['screen', 'tree', 'tap', 'swipe', 'health', 'draft'].includes(action) &&
      ['--id', '--direction', '--timeout', '--text'].some(flag => values.has(flag))) {
    throw new Error('--id / --direction / --timeout 只支持真机调试命令');
  }
  if ((enabled.has('--reset-data') || enabled.has('--confirm-reset')) && !['flash', 'dev'].includes(action)) {
    throw new Error('清空设备参数只能用于 flash / dev');
  }
  if (enabled.has('--dry-run') && action !== 'flash') throw new Error('--dry-run 只支持 flash，不会构建或连接设备');
  const config = await loadDeviceConfig();
  if (action === 'setup') {
    const emsdk = values.get('--emsdk');
    if (emsdk) {
      await readFile(path.resolve(emsdk, 'upstream/emscripten/emcc.py'));
      config.emsdk = path.resolve(emsdk);
    }
    const idf = values.get('--idf');
    if (idf) {
      await readFile(path.resolve(idf, 'tools/cmake/project.cmake'));
      config.idf = path.resolve(idf);
    }
    const port = values.get('--port');
    if (port) config.port = port;
    if (!emsdk && !port && !idf) throw new Error('通过 --emsdk / --idf / --port 指定需要保存的本机配置');
    await writeFile(configFile, JSON.stringify(config, null, 2) + '\n');
    console.log('已保存 editor/device.local.json（Git 忽略，不保存令牌）');
    return;
  }
  if (action === 'ports') {
    console.table(await listSerialPorts());
    return;
  }
  if (values.has('--bridge')) {
    if (!['screen', 'tree', 'tap', 'swipe', 'health', 'draft'].includes(action) || values.has('--port')) throw new Error('--bridge 只支持实时调试，且不能同时 --port');
    const debugFlags = [action, '--bridge', values.get('--bridge')!];
    for (const name of ['--id', '--direction', '--timeout', '--text']) {
      const value = values.get(name); if (value !== undefined) debugFlags.push(name, value);
    }
    await runTool('python', ['-X', 'utf8', path.join(editorRoot, 'device-debug.py'), ...debugFlags], {cwd: editorRoot, inherit: true});
    return;
  }
  const requested = values.get('--port') ?? process.env.ESPFLASH_PORT ?? config.port;
  // A dry run only prints a validated plan; no device enumeration or serial connection.
  const port = enabled.has('--dry-run') ? requested ?? '<USB-port>' : selectSerialPort(await listSerialPorts(), requested);
  if (['screen', 'tree', 'tap', 'swipe', 'health'].includes(action)) {
    const debugFlags = [action, '--port', port];
    for (const name of ['--id', '--direction', '--timeout']) {
      const value = values.get(name);
      if (value) debugFlags.push(name, value);
    }
    await runTool('python', ['-X', 'utf8', path.join(editorRoot, 'device-debug.py'), ...debugFlags],
      {cwd: editorRoot, inherit: true});
    return;
  }
  const resetData = enabled.has('--reset-data');
  let confirmReset = enabled.has('--confirm-reset');
  if (resetData && !confirmReset && !enabled.has('--dry-run')) {
    if (!process.stdin.isTTY) throw new Error('清空设备需 --confirm-reset；将删除 Wi-Fi、配对身份、空间状态和布局');
    const prompt = createInterface({input: process.stdin, output: process.stdout});
    try {
      confirmReset = (await prompt.question(`将清空 ${port} 的全部 Flash，删除 Wi-Fi、配对身份、空间状态和布局。输入 ERASE 确认：`)) === 'ERASE';
    } finally { prompt.close(); }
    if (!confirmReset) throw new Error('已取消；未擦除或烧录');
  }
  if (action === 'dev') {
    const buildFlags = ['--firmware'];
    for (const name of ['--emsdk', '--idf']) {
      const value = values.get(name);
      if (value) buildFlags.push(name, value);
    }
    // Build failure aborts before any flash operation; no fallback to an old image.
    await runTool(process.execPath, ['--experimental-strip-types', path.join(editorRoot, 'src/build.mts'), ...buildFlags],
      {cwd: editorRoot, inherit: true});
  }
  if (action === 'flash' || action === 'dev') {
    const plan = await buildFlashPlan(port, {resetData, confirmReset: confirmReset || enabled.has('--dry-run')});
    if (enabled.has('--dry-run')) {
      console.log(JSON.stringify({port, resetData, commands: plan}, null, 2));
      return;
    }
    console.log(resetData ? '清空设备并安装固件…' : '清理程序分区并更新固件；保留 NVS 和布局…');
    await executeFlashPlan(plan, (command, commandArgs) => runTool(command, commandArgs, {inherit: true}));
    console.log(resetData ? '安装完成，需要重新配置 Wi-Fi 和配对。' : '更新完成，Wi-Fi/配对/空间/布局已保留。');
    if (action === 'flash' || enabled.has('--no-monitor')) return;
  }
  const monitor = ['monitor', '--port', port, '--monitor-baud', '115200', '--non-interactive', '--no-reset', '--skip-update-check'];
  // Only dev knows its freshly built ELF matches the just-flashed device.
  if (action === 'dev') monitor.push('--elf', firmwareElf);
  await runTool('espflash', monitor, {inherit: true});
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  deviceMain(process.argv.slice(2)).catch(error => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
