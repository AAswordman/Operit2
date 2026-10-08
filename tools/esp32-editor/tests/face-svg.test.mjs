import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compileFaceSvg} from '../src/face-svg.mts';

const source = await readFile(new URL('../../../apps/esp32/ui_port/face.svg', import.meta.url), 'utf8');
test('face SVG compiles deterministically to the checked-in flash primitives',async()=>{
  assert.equal(compileFaceSvg(source),(await readFile(new URL('../../../apps/esp32/ui_port/mini_face.h',import.meta.url),'utf8')).replaceAll('\r\n', '\n'));
  assert.equal(compileFaceSvg(source).match(/\{[01],/g).length,6);
});
test('SVG compiler rejects unsupported elements, paths, colors and oversized geometry',()=>{
  for(const invalid of [source.replace('<ellipse','<image'),source.replace('M 56 68','Q 56 68'),
    source.replace('#b1f2d8','url(#gradient)'),source.replace('rx="10"','rx="1000"'),
    source.replace('stroke-width="4"','stroke-width="0"'),source.replace('fill="#b1f2d8"','transform="translate(4)" fill="#b1f2d8"')]) {
    assert.throws(()=>compileFaceSvg(invalid));
  }
});


test('SVG compiler rejects unsupported root attributes instead of ignoring their effects', () => {
  for (const attribute of ['transform="translate(50 0)"', 'style="opacity: 0.5"',
    'width="320"', "transform='translate(50 0)'", 'xmlns="urn:not-svg"']) {
    assert.throws(() => compileFaceSvg(source.replace('<svg ', `<svg ${attribute} `)));
  }
});

test('SVG compiler consumes the complete attribute text and rejects duplicate attributes', () => {
  for (const invalid of [
    source.replace('<svg ', '<svg transform=translate(50) '),
    source.replace('<svg ', '<svg stray-text '),
    source.replace('<svg ', '<svg viewBox="0 0 160 100" '),
    source.replace('<ellipse ', '<ellipse transform=translate(50) '),
    source.replace('<ellipse ', "<ellipse transform='translate(50)' "),
    source.replace('<ellipse ', '<ellipse cx="48" '),
    source.replace('<ellipse ', '<ellipse stray-text '),
    source.replace('cx="48" cy=', 'cx="48"cy='),
    source.replace('<path ', '<path stroke-width="4" '),
    source.replace('<path ', '<path transform=translate(50) '),
    source.replace('</svg>', '</svg><svg viewBox="0 0 160 100"></svg>'),
    `stray-text${source}`,
  ]) {
    assert.throws(() => compileFaceSvg(invalid));
  }
});

test('SVG compiler accepts supported quoted attributes without changing flash primitives', () => {
  const expected = compileFaceSvg(source);
  assert.equal(compileFaceSvg(source.replaceAll('"', "'")), expected);
  assert.equal(compileFaceSvg(source.replaceAll('="', ' = "')), expected);
  assert.equal(compileFaceSvg(`<!-- <svg transform="translate(50)"> -->${source}`), expected);
});
