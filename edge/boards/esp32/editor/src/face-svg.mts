/** Compile our deliberately small SVG vocabulary into flash-resident primitives.
 * Unsupported SVG is rejected, never silently rendered differently on the board. */
export function compileFaceSvg(source: string): string {
  // Consume every attribute, including its separator and quotes. Partial regex
  // extraction would silently drop malformed, unquoted or duplicate attributes.
  const attributes = (text: string, allowed: string[]): Record<string, string> => {
    const attrs: Record<string, string> = Object.create(null);
    const attribute = /\s+([A-Za-z_][\w:.-]*)\s*=\s*(?:"([^"<>]*)"|'([^'<>]*)')/y;
    let offset = 0;
    while (text.slice(offset).trim()) {
      attribute.lastIndex = offset;
      const match = attribute.exec(text);
      if (!match) throw new Error('Malformed face SVG attribute');
      const key = match[1];
      if (!allowed.includes(key) || Object.hasOwn(attrs, key)) throw new Error('Unsupported or duplicate face SVG attribute');
      attrs[key] = match[2] ?? match[3];
      offset = attribute.lastIndex;
    }
    return attrs;
  };
  const svg = /^<svg\b([^>]*)>([\s\S]*)<\/svg>$/.exec(source.replace(/<!--[\s\S]*?-->/g, '').trim());
  if (!svg) throw new Error('Face SVG requires one svg root');
  const root = attributes(svg[1], ['xmlns', 'viewBox']);
  if (root.viewBox !== '0 0 160 100') throw new Error('Face SVG requires viewBox 0 0 160 100');
  if (root.xmlns !== undefined && root.xmlns !== 'http://www.w3.org/2000/svg') throw new Error('Unsupported face SVG namespace');
  const body = svg[2].trim();
  const tags = [...body.matchAll(/<(ellipse|path)\b([^>]*)\/>/g)];
  if (!tags.length || body.replace(/<(ellipse|path)\b[^>]*\/>/g, '').trim()) throw new Error('Unsupported face SVG element');
  const primitives: string[] = [];
  const integer = (value: string | undefined, limit = 160) => {
    if (!value || !/^\d+$/.test(value) || Number(value) > limit) throw new Error('Invalid face SVG coordinate');
    return Number(value);
  };
  const color = (value: string | undefined) => {
    if (!value || !/^#[0-9a-f]{6}$/i.test(value)) throw new Error('Face SVG requires a solid RGB color');
    return '0x' + value.slice(1);
  };
  for (const [, tag, text] of tags) {
    const allowed = tag === 'ellipse' ? ['cx', 'cy', 'rx', 'ry', 'fill'] : ['d', 'fill', 'stroke', 'stroke-width'];
    const attrs = attributes(text, allowed);
    if (tag === 'ellipse') {
      const values = ['cx', 'cy', 'rx', 'ry'].map(key => integer(attrs[key]));
      if (!values[2] || !values[3] || values[0] - values[2] < 0 || values[0] + values[2] > 160 ||
          values[1] - values[3] < 0 || values[1] + values[3] > 100) throw new Error('Face ellipse is out of bounds');
      primitives.push(`    {0, ${values.join(', ')}, 0, ${color(attrs.fill)}}`);
    } else {
      if (attrs.fill !== 'none') throw new Error('Face path must be unfilled');
      const tokens = attrs.d?.match(/[ML]|\d+/g) ?? [];
      if (!attrs.d || attrs.d.replace(/[ML]|\d+|\s+/g, '') || tokens.length < 6 || tokens.length % 3) throw new Error('Face paths support M/L only');
      let previous: number[] = [];
      const width = integer(attrs['stroke-width'], 8);
      if (!width) throw new Error('Face stroke width must be positive');
      for (let i = 0; i < tokens.length; i += 3) {
        const point = [integer(tokens[i + 1]), integer(tokens[i + 2], 100)];
        if (tokens[i] !== (i ? 'L' : 'M')) throw new Error('Face path must start with M and continue with L');
        if (i) primitives.push(`    {1, ${[...previous, ...point].join(', ')}, ${width}, ${color(attrs.stroke)}}`);
        previous = point;
      }
    }
  }
  if (primitives.length > 32) throw new Error('Face SVG exceeds 32 primitives');
  return '/* Generated from face.svg by face-svg.mts. Do not edit. */\n' +
    'static const face_primitive_t face_primitives[] = {\n' + primitives.join(',\n') + '\n};\n';
}
