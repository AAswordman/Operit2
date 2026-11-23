import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';

const script = readFileSync(new URL('../../apps/flutter/thirdparty/webview_flutter_android/android/src/main/assets/operit_web_visit.js', import.meta.url), 'utf8');

/** Models only DOM access; any accidental dependency on painted text fails the test. */
function element(tagName, attrs = {}, ...children) {
  const node = {nodeType: 1, tagName, hidden: !!attrs.hidden, childNodes: children.map(
    child => typeof child === 'string' ? {nodeType: 3, nodeValue: child} : child),
    getAttribute: name => attrs[name] ?? null, attrs};
  Object.defineProperty(node, 'innerText', {get() { throw Error('Rendering-dependent innerText is forbidden'); }});
  return node;
}

/** Executes the production extractor with a private page base URL and no window timers. */
function extract({body, links = [], images = [], meta = [], captcha = false, readyState = 'complete'}) {
  const document = {body, title: 'Example', baseURI: 'https://example.test/base/', readyState,
    querySelectorAll: selector => ({meta, 'a[href]': links, img: images})[selector],
    querySelector: () => captcha ? {} : null};
  return JSON.parse(vm.runInNewContext(script, {
    document, location: {href: 'https://example.test/page'}, URL,
    getComputedStyle: node => ({display: node.attrs.display ?? 'block', visibility: node.attrs.visibility ?? 'visible'}),
  }));
}

test('extracts DOM text and block boundaries without page frames, scrolling or innerText', () => {
  const body = element('BODY', {}, element('H1', {}, 'Title'), element('P', {}, 'First ', element('B', {}, 'bold'), ' sentence.'),
    element('SCRIPT', {}, 'secret code'), element('STYLE', {}, 'style code'),
    element('DIV', {hidden: true}, 'hidden text'), element('DIV', {'aria-hidden': 'true'}, 'aria text'),
    element('DIV', {display: 'none'}, 'display text'), element('DIV', {visibility: 'hidden'}, 'invisible text'));
  assert.equal(extract({body}).result.content, 'Title\nFirst bold sentence.');
});

test('resolves relative URLs against document.baseURI and preserves distinct link labels', () => {
  const links = [element('A', {href: '../next'}, 'Next'), element('A', {href: '../next'}, 'Next'),
    element('A', {href: '../next'}, 'Other label'), element('A', {href: 'javascript:alert(1)'}, 'Execute')];
  const images = [element('IMG', {src: 'image.png'}), element('IMG', {src: 'image.png'}), element('IMG', {src: 'data:bad'})];
  const result = extract({body: element('BODY', {}, 'text'), links, images}).result;
  assert.deepEqual(result.links, [{url: 'https://example.test/next', text: 'Next'}, {url: 'https://example.test/next', text: 'Other label'}]);
  assert.deepEqual(result.imageLinks, ['https://example.test/base/image.png']);
});

test('keeps challenge pages waiting and does not classify a normal mention of captcha as a challenge', () => {
  assert.equal(extract({body: element('BODY', {}, 'An article about captcha')}).captcha, false);
  assert.equal(extract({body: element('BODY', {}, 'Please verify'), captcha: true}).captcha, true);
  assert.equal(extract({body: element('BODY', {}, 'Loading'), readyState: 'interactive'}).ready, false);
});

test('metadata keys cannot mutate the result prototype', () => {
  const meta = [element('META', {name: '__proto__', content: 'metadata'}), element('META', {name: 'description', content: 'summary'})];
  const result = extract({body: element('BODY', {}, 'text'), meta}).result;
  assert.equal(result.metadata.__proto__, 'metadata');
  assert.equal(result.metadata.description, 'summary');
});
