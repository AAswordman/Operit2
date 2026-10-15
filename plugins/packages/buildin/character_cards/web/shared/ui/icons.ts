const ICONS = {
  person: '<circle cx="12" cy="8" r="3.5"/><path d="M5 21v-2a7 7 0 0 1 14 0v2"/>',
  badge: '<rect x="4" y="4" width="16" height="17" rx="2"/><path d="M9 4V2h6v2M8 16h8"/><circle cx="12" cy="10" r="2"/>',
  groups: '<circle cx="9" cy="8" r="3"/><path d="M3 20v-2a6 6 0 0 1 12 0v2M16 5a3 3 0 0 1 0 6M18 14a5 5 0 0 1 3 5"/>',
  add: '<path d="M12 5v14M5 12h14"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  edit: '<path d="m14 4 6 6M4 20l4-1 12-12-5-5L3 14z"/>',
  graph: '<rect x="9" y="2" width="6" height="6" rx="1"/><rect x="2" y="16" width="6" height="6" rx="1"/><rect x="16" y="16" width="6" height="6" rx="1"/><path d="M12 8v4M5 16v-4h14v4"/>',
  profile: '<rect x="4" y="3" width="16" height="19" rx="2"/><circle cx="12" cy="9" r="3"/><path d="M7 18v-1a5 5 0 0 1 10 0v1"/>',
  tune: '<path d="M4 6h7M15 6h5M4 12h2M10 12h10M4 18h10M18 18h2"/><circle cx="13" cy="6" r="2"/><circle cx="8" cy="12" r="2"/><circle cx="16" cy="18" r="2"/>',
  hub: '<circle cx="12" cy="12" r="3"/><circle cx="4" cy="4" r="2"/><circle cx="20" cy="4" r="2"/><circle cx="4" cy="20" r="2"/><circle cx="20" cy="20" r="2"/><path d="m6 6 4 4m4 4 4 4m-12 0 4-4m4-4 4-4"/>',
  refresh: '<path d="M20 10a8 8 0 1 0 0 5M20 4v6h-6"/>',
  upload: '<path d="M12 16V3m-5 5 5-5 5 5M4 15v5h16v-5"/>',
  download: '<path d="M12 3v13m-5-5 5 5 5-5M4 15v5h16v-5"/>',
  check: '<path d="m5 12 4 4 10-10"/>',
  trash: '<path d="M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7M14 10v7"/>',
  expand: '<path d="M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7v.5"/>',
  tools: '<path d="m15 4 3 3 3-3a6 6 0 0 1-7 8L6 21l-3-3 9-8a6 6 0 0 1 3-6"/>',
  search: '<circle cx="10" cy="10" r="6"/><path d="m15 15 6 6"/>',
  memory: '<path d="M4 5h16v14H4zM8 1v4m8-4v4M8 19v4m8-4v4M1 9h3m-3 6h3m16-6h3m-3 6h3"/><rect x="8" y="9" width="8" height="6" rx="1"/>',
  folder: '<path d="M3 6h7l2 3h9v12H3z"/>',
  link: '<path d="m9 15 6-6M8 16l-1 1a4 4 0 0 1-6-6l5-5a4 4 0 0 1 6 0m0 12a4 4 0 0 0 6 0l5-5a4 4 0 0 0-6-6l-1 1"/>',
  up: '<path d="m6 14 6-6 6 6"/>',
  down: '<path d="m6 10 6 6 6-6"/>',
  minus: '<path d="M5 12h14"/>',
  back: '<path d="M19 12H5m7-7-7 7 7 7"/>',
  pin: '<path d="m8 3 8 0-1 7 3 4H6l3-4zM12 14v8"/>',
  lock: '<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V6a4 4 0 0 1 8 0v4M12 14v3"/>',
  move: '<path d="M3 12h18m-4-4 4 4-4 4M7 8l-4 4 4 4"/>',
  chat: '<path d="M4 3h16v14H9l-5 4zM8 7h8M8 11h5"/>',
  more: '<circle cx="12" cy="5" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="12" cy="19" r="1"/>',
};
export type IconName = keyof typeof ICONS;

/** Returns a declared local vector icon without downloading fonts or assets. */
export function icon(name: IconName): string {
  if (!Object.hasOwn(ICONS, name)) throw new Error(`Unknown icon: ${name}`);
  return `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${ICONS[name]}</svg>`;
}
