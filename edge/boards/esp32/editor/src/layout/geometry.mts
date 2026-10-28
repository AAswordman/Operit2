export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Resizes a node from a named corner while keeping children inside and staying on the board. */
export function resizeFromCorner(
  node: Box,
  corner: string,
  dx: number,
  dy: number,
  width: number,
  height: number,
  children: Box[] = [],
): Box {
  const minW = Math.max(8, ...children.map((child) => child.x + child.w));
  const minH = Math.max(8, ...children.map((child) => child.y + child.h));
  let {x, y, w, h} = node;
  if (corner.includes('w')) {
    x = Math.max(0, Math.min(node.x + dx, node.x + node.w - minW));
    w = node.x + node.w - x;
  } else {
    w = Math.max(minW, Math.min(width - x, node.w + dx));
  }
  if (corner.includes('n')) {
    y = Math.max(0, Math.min(node.y + dy, node.y + node.h - minH));
    h = node.y + node.h - y;
  } else {
    h = Math.max(minH, Math.min(height - y, node.h + dy));
  }
  return {x, y, w, h};
}
