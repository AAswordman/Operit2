import {query} from './types.js';

type PanelName = 'canvas' | 'components' | 'properties' | 'layers' | 'device';
type InspectorTab = Exclude<PanelName, 'canvas' | 'components'>;

const mobile = window.matchMedia('(max-width:760px)');
const stage = query<HTMLElement>('.stage');
const device = query<HTMLElement>('.device');
const zoomSelect = query<HTMLSelectElement>('#zoom');
let activeTab: InspectorTab = 'properties';

/** Changes the visible editor panel and optionally scrolls to it on mobile. */
export function showPanel(name: PanelName, scroll = false): void {
  document.body.dataset.panel = name;
  if (name === 'properties' || name === 'layers' || name === 'device') {
    activeTab = name;
  }
  document.querySelectorAll<HTMLElement>('.inspector [data-pane]').forEach((pane) => {
    pane.hidden = pane.dataset.pane !== activeTab;
  });
  document.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach((button) => {
    button.setAttribute('aria-pressed', String(button.dataset.tab === activeTab));
  });
  document.querySelectorAll<HTMLButtonElement>('[data-mobile-panel]').forEach((button) => {
    button.setAttribute('aria-pressed', String(button.dataset.mobilePanel === name));
  });
  if (scroll && mobile.matches) {
    const target = name === 'canvas' ? query<HTMLElement>('.preview') : document.querySelector<HTMLElement>(`[data-pane="${name}"]`);
    target?.scrollIntoView({
      block: 'start',
      behavior: window.matchMedia('(prefers-reduced-motion:reduce)').matches ? 'instant' : 'smooth',
    });
  }
}

/** Calculates and applies the canvas scale for the current stage and zoom setting. */
function fitCanvas(): void {
  const style = getComputedStyle(stage);
  const available = Math.max(
    100,
    stage.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 10,
  );
  const fit = Math.min(
    available / 320,
    mobile.matches ? 2 : Math.max(0.3, (stage.clientHeight - 100) / 240),
    3,
  );
  device.style.setProperty('--scale', zoomSelect.value === 'auto' ? String(fit) : zoomSelect.value);
}

/** Handles a tab button click. */
function handleInspectorTab(event: Event): void {
  const button = event.currentTarget;
  if (!(button instanceof HTMLButtonElement)) throw new Error('检查面板按钮类型错误');
  const name = button.dataset.tab;
  if (!name || !['properties', 'layers', 'device'].includes(name)) throw new Error('检查面板标签无效');
  showPanel(name as InspectorTab);
}

/** Handles a mobile panel button click. */
function handleMobilePanel(event: Event): void {
  const button = event.currentTarget;
  if (!(button instanceof HTMLButtonElement)) throw new Error('移动面板按钮类型错误');
  const name = button.dataset.mobilePanel;
  if (!name || !['canvas', 'components', 'properties', 'layers', 'device'].includes(name)) {
    throw new Error('移动面板标签无效');
  }
  showPanel(name as PanelName, true);
}

document.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach((button) => {
  button.addEventListener('click', handleInspectorTab);
});
document.querySelectorAll<HTMLButtonElement>('[data-mobile-panel]').forEach((button) => {
  button.addEventListener('click', handleMobilePanel);
});
query<HTMLButtonElement>('#inspect-node').addEventListener('click', () => showPanel('properties', true));
window.addEventListener('operit-component-added', () => {
  if (mobile.matches) showPanel('canvas', true);
});
zoomSelect.addEventListener('change', fitCanvas);
new ResizeObserver(fitCanvas).observe(stage);
mobile.addEventListener('change', () => {
  showPanel(mobile.matches ? 'canvas' : activeTab);
  fitCanvas();
});
showPanel('canvas');
fitCanvas();
