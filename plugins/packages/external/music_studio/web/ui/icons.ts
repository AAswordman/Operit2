const paths = {
  library: '<path d="M4 4h16v16H4zM4 8h16M8 8v12"/>',
  mixer: '<path d="M5 3v7m0 4v7M12 3v12m0 4v2M19 3v3m0 4v11M2 10h6M9 15h6M16 6h6"/>',
  spark: '<path d="m12 3 2.8 6.2L21 12l-6.2 2.8L12 21l-2.8-6.2L3 12l6.2-2.8z"/>',
  arrangement: '<path d="M3 5h5v14H3zM11 5h10v4H11zM11 12h6v3h-6zM11 18h10"/>',
  play: '<path d="m8 5 11 7-11 7z" fill="currentColor" stroke="none"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" stroke="none"/>',
  loop: '<path d="M4 9a4 4 0 0 1 4-4h12m-4-3 4 3-4 3M20 15a4 4 0 0 1-4 4H4m4 3-4-3 4-3"/>',
  undo: '<path d="m8 4-5 5 5 5M3 9h11a6 6 0 0 1 0 12"/>',
  redo: '<path d="m16 4 5 5-5 5M21 9H10a6 6 0 0 0 0 12"/>',
  wave: '<path d="M2 12h3l3-8 5 16 3-8h6"/>',
} as const;
export function icon(name: keyof typeof paths): string {
  return `<svg class="ui-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`;
}
