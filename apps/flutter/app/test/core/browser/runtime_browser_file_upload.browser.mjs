/** Exercises the production chooser runtime in a real browser without building Rust. */
export async function verifyBrowserFileUpload(page, runtimeScript) {
  await page.goto('about:blank');
  await page.setContent(`
    <input id="single" type="file">
    <input id="multiple" type="file" multiple>
    <input id="directory" type="file" webkitdirectory>
    <input id="manual" type="file">
    <button id="trigger" onclick="document.getElementById('single').click()">Upload</button>
    <iframe id="frame" srcdoc="<input id='inside' type='file'>"></iframe>
  `);
  await page.waitForFunction(() => document.getElementById('frame').contentDocument?.getElementById('inside'));
  await page.evaluate(runtimeScript);

  const checks = await page.evaluate(async () => {
    const runtime = window.__operitBrowserFileChooser;
    const single = document.getElementById('single');
    const multiple = document.getElementById('multiple');
    const file = { name: '中文.txt', base64: btoa('browser upload'), type: 'text/plain' };
    const results = [];

    /** Records a successful invariant or stops the suite at its first failure. */
    function check(condition, label) {
      if (!condition) throw new Error(label);
      results.push(label);
    }

    /** Requires the exact expected failure without accepting unrelated errors. */
    function expectError(action, message) {
      try {
        action();
      } catch (error) {
        check(error.message === message, message);
        return;
      }
      throw new Error(`Expected failure: ${message}`);
    }

    /** Runs chooser completion with the same interception scope as the host. */
    function upload(files) {
      return runtime.run(() => runtime.upload(files));
    }

    const events = [];
    for (const type of ['input', 'change', 'cancel']) {
      single.addEventListener(type, event => events.push(event.type));
    }
    runtime.run(() => document.getElementById('trigger').click());
    check(runtime.pendingInput === single, 'button-triggered chooser is captured');
    upload([file]);
    check(await single.files[0].text() === 'browser upload', 'uploaded bytes are preserved');
    check(single.files[0].name === '中文.txt' && single.files[0].type === 'text/plain', 'file name and MIME type are preserved');
    check(events.join(',') === 'input,change', 'selection emits input and change in order');
    check(runtime.pendingInput === null, 'successful selection consumes the chooser');
    expectError(() => upload([file]), 'No active browser file chooser');

    runtime.run(() => single.click());
    expectError(() => upload([file, file]), 'Browser file chooser does not allow multiple files');
    check(runtime.pendingInput === single && single.files.length === 1, 'failed selection preserves chooser and existing files');
    upload([file]);
    check(runtime.pendingInput === null, 'corrected selection succeeds on the retained chooser');

    const selected = single.files[0];
    const count = events.length;
    runtime.run(() => single.click());
    upload(null);
    check(single.files[0] === selected, 'cancellation preserves the existing selection');
    check(events.slice(count).join(',') === 'cancel', 'cancellation emits only cancel');
    runtime.run(() => single.click());
    upload([]);
    check(single.files.length === 0 && events.slice(-2).join(',') === 'input,change', 'empty selection clears files and notifies the page');

    runtime.run(() => multiple.click());
    upload([file, { name: 'binary.bin', base64: btoa(String.fromCharCode(0, 255, 10)) }]);
    const binary = new Uint8Array(await multiple.files[1].arrayBuffer());
    check(multiple.files.length === 2 && Array.from(binary).join(',') === '0,255,10', 'multiple uploads preserve binary data');

    const detached = document.createElement('input');
    detached.type = 'file';
    runtime.run(() => detached.click());
    upload([file]);
    check(detached.files[0].name === file.name, 'detached programmatic file inputs work');

    const shadow = document.createElement('div').attachShadow({ mode: 'open' });
    shadow.innerHTML = '<input type="file">';
    const shadowInput = shadow.querySelector('input');
    runtime.run(() => shadowInput.click());
    upload([file]);
    check(shadowInput.files.length === 1, 'shadow-root file inputs work');

    const frame = document.getElementById('frame');
    const inside = frame.contentDocument.getElementById('inside');
    runtime.run(() => inside.click());
    upload([file]);
    check(await inside.files[0].text() === 'browser upload', 'same-origin frame file inputs work');

    await runtime.run(async () => {
      await Promise.resolve();
      single.click();
      return 42;
    });
    check(runtime.pendingInput === single && runtime.captureDepth === 0, 'awaited page actions retain their capture scope');
    upload([file]);
    check(runtime.run(() => 42) === 42, 'synchronous evaluation preserves its result');
    expectError(() => runtime.run(() => { throw new Error('action failed'); }), 'action failed');
    check(runtime.captureDepth === 0, 'failed actions release their capture scope');

    expectError(() => runtime.run(() => ({
      get then() { throw new Error('then failed'); },
    })), 'then failed');
    check(runtime.captureDepth === 0, 'thenable failures release their capture scope');
    try {
      await runtime.run(() => Promise.reject(new Error('async failed')));
      throw new Error('Expected asynchronous failure');
    } catch (error) {
      check(error.message === 'async failed' && runtime.captureDepth === 0, 'rejected page promises release their capture scope');
    }

    runtime.run(() => single.showPicker());
    upload([file]);
    check(single.files.length === 1, 'explicit showPicker requests work');
    runtime.run(() => document.getElementById('directory').click());
    expectError(() => upload([file]), 'Browser upload does not support directory choosers');
    upload(null);

    runtime.run(() => single.click());
    single.disabled = true;
    expectError(() => upload([file]), 'Browser file input is disabled');
    check(runtime.pendingInput === single, 'disabled-input failures retain chooser state');
    single.disabled = false;
    upload(null);

    runtime.run(() => inside.click());
    const loaded = new Promise(resolve => frame.addEventListener('load', resolve, { once: true }));
    frame.srcdoc = '<input id="replacement" type="file">';
    await loaded;
    expectError(() => upload([file]), 'Browser file chooser is no longer valid');
    const replacement = frame.contentDocument.getElementById('replacement');
    runtime.run(() => replacement.click());
    upload([file]);
    check(replacement.files.length === 1, 'frame navigation installs a fresh chooser lifecycle');
    return results;
  });

  return checks;
}

/** Verifies trusted manual selection remains outside the automation capture scope. */
export async function verifyManualFileSelection(page) {
  const [manualChooser] = await Promise.all([
    page.waitForEvent('filechooser'),
    page.locator('#manual').click(),
  ]);
  await manualChooser.setFiles([]);
  const pending = await page.evaluate(() => window.__operitBrowserFileChooser.pendingInput);
  if (pending !== null) throw new Error('Manual file selection was intercepted');
}

/** Verifies native script injection on a page that explicitly forbids unsafe-eval. */
export async function verifyBrowserFileUploadCsp(page, runtimeScript) {
  await page.goto('about:blank');
  await page.setContent(`
    <meta http-equiv="Content-Security-Policy" content="script-src 'self'">
    <input id="file" type="file">
    <button id="button">Click</button>
  `);
  const session = await page.context().newCDPSession(page);
  const checks = [];
  let captureId = 0;

  /** Uses native injection while keeping CSP restrictions on page-owned eval. */
  async function inject(expression) {
    const response = await session.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise: true,
      allowUnsafeEvalBlockedByCSP: false,
    });
    if (response.exceptionDetails) {
      throw new Error(response.exceptionDetails.exception?.description ?? response.exceptionDetails.text);
    }
    return response.result.value;
  }

  /** Mirrors the host's separate capture, native execution, and cleanup calls. */
  async function evaluate(script) {
    const scope = captureId++;
    await inject(`${runtimeScript}\nwindow.__operitBrowserFileChooser.beginCapture(${scope});`);
    try {
      return await inject(script);
    } finally {
      await inject(`window.__operitBrowserFileChooser?.endCapture(${scope});`);
    }
  }

  function check(condition, label) {
    if (!condition) throw new Error(label);
    checks.push(label);
  }

  try {
    let evalBlocked = false;
    try {
      await inject("(0, eval)('1 + 1')");
    } catch (error) {
      evalBlocked = error.message.includes('EvalError') && error.message.includes('Content Security Policy');
    }
    check(evalBlocked, 'strict CSP rejects the original indirect-eval path');
    check(await evaluate('1 + 1') === 2, 'native evaluation succeeds under strict CSP');
    check((await evaluate('({width: innerWidth})')).width > 0, 'object results survive strict CSP');
    check(await evaluate('var __cspGlobal = 40; __cspGlobal + 2;') === 42, 'statement completion values are preserved');
    check(await evaluate('window.__cspGlobal') === 40, 'global declarations survive later calls');
    await inject("document.getElementById('button').addEventListener('click', () => window.__clicked = true);");
    await evaluate("document.getElementById('button').click();");
    check(await inject('window.__clicked') === true, 'click handlers execute under strict CSP');
    check(await evaluate("(async () => { await Promise.resolve(); document.getElementById('file').click(); return 42; })()") === 42,
      'asynchronous file chooser actions preserve their results');
    check(await inject("window.__operitBrowserFileChooser.pendingInput === document.getElementById('file')"),
      'asynchronous file chooser requests are intercepted');
    await inject(`window.__operitBrowserFileChooser.run(() => window.__operitBrowserFileChooser.upload([
      {name: 'csp.txt', base64: btoa('CSP upload'), type: 'text/plain'}
    ]));`);
    check(await inject("document.getElementById('file').files[0].text()") === 'CSP upload',
      'file uploads still work under strict CSP');
    for (const script of ['const = invalid', 'throw new Error("page failure")']) {
      let failed = false;
      try {
        await evaluate(script);
      } catch {
        failed = true;
      }
      check(failed && await inject('window.__operitBrowserFileChooser.captureDepth') === 0,
        `errors propagate and release capture: ${script}`);
    }
    return checks;
  } finally {
    await session.detach();
  }
}
