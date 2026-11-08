import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const enabled = process.env.OPERIT_ANDROID_TERMINAL_TEST === '1';

/** Exercises the production mksh initialization on a connected Android PTY without installing an app. */
test('Android interactive shell survives interrupts and preserves status, directory and environment', { skip: !enabled, timeout: 20000 }, async () => {
  const child = spawn('adb', ['shell', '-tt', 'stty rows 24 cols 80; exec /system/bin/sh -i'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let output = '', failure = null, wake = () => {};
  child.stdout.on('data', bytes => { output += bytes.toString(); wake(); });
  child.stderr.on('data', bytes => { failure = new Error(bytes.toString()); wake(); });
  child.on('error', error => { failure = error; wake(); });
  child.on('exit', code => { failure = new Error(`Android shell exited: ${code}`); wake(); });
  /** Waits for actual shell output with a deadline, retaining bytes across fragmented reads. */
  async function waitFor(pattern, timeout = 4000) {
    const deadline = Date.now() + timeout;
    for (;;) {
      const match = pattern.exec(output);
      if (match) { output = output.slice(match.index + match[0].length); return match; }
      if (failure) throw failure;
      if (Date.now() >= deadline) throw new Error(`Missing ${pattern}: ${JSON.stringify(output)}`);
      await new Promise(resolve => {
        const timer = setTimeout(resolve, Math.min(50, deadline - Date.now()));
        wake = () => { clearTimeout(timer); resolve(); };
      });
    }
  }
  /** Waits for the private completion marker and decodes its directory and exit status. */
  async function prompt() {
    const marker = await waitFor(/\x1b\]133;OperitPrompt=([^:]*):(-?\d+)\x07/);
    return { directory: Buffer.from(marker[1], 'base64').toString(), status: Number(marker[2]) };
  }
  /** Sends one complete shell submission through the actual Android controlling terminal. */
  function send(command) { child.stdin.write(command + '\n'); }
  try {
    child.stdin.write(readFileSync(new URL('hosts/android/src/system_shell_init.sh', root)));
    await prompt();
    send('cd /data/local/tmp; export OPERIT_TERMINAL_CHECK=kept; (exit 7)');
    assert.deepEqual(await prompt(), { directory: '/data/local/tmp', status: 7 });

    send("printf '\\nRUNNING\\n'; sleep 30; printf 'SHOULD_NOT_RUN\\n'");
    await waitFor(/\r*\nRUNNING\r*\n/);
    // Let mksh transfer the controlling terminal from printf to the sleep job.
    await new Promise(resolve => setTimeout(resolve, 100));
    child.stdin.write('\x03');
    assert.equal((await prompt()).status, 130);
    assert.doesNotMatch(output, /\r*\nSHOULD_NOT_RUN\r*\n/);

    // Idle Ctrl+C must leave a usable shell too.
    child.stdin.write('\x03');
    await prompt();
    send("printf '\\nSTATE:%s:%s\\n' \"$PWD\" \"$OPERIT_TERMINAL_CHECK\"; (exit 9)");
    await waitFor(/\r*\nSTATE:\/data\/local\/tmp:kept\r*\n/);
    assert.deepEqual(await prompt(), { directory: '/data/local/tmp', status: 9 });

    // Two logical lines encoded as one submission must produce only the final completion.
    send("eval \"$(printf '%b' 'false\\0012printf multiline')\"; (exit 3)");
    assert.equal((await prompt()).status, 3);
  } finally {
    child.stdin.write('exit\n');
    child.stdin.end();
    child.kill();
  }
});
