import { createAvatarSources } from "./avatar-sources";

/** Starts images after the page can paint, then patches only avatar elements. */
export function createAvatarLoader(
  root: HTMLElement,
  sources: Map<string, string>,
  resolve: (path: string | null) => Promise<string>,
) {
  const avatars = createAvatarSources(sources, resolve);
  let scheduled = false;
  let paths: readonly (string | null)[] = [];

  function display(path: string | null, source: string): void {
    for (const element of root.querySelectorAll<HTMLElement>("[data-avatar-uri]")) {
      if (element.dataset.avatarUri !== (path ?? "")) continue;
      const existing = element.querySelector("img");
      if (existing !== null) {
        if (existing.getAttribute("src") !== source) existing.src = source;
        continue;
      }
      const image = document.createElement("img");
      image.alt = "";
      image.onload = () => {
        if (!element.isConnected || element.dataset.avatarUri !== (path ?? "")) return;
        if (element.querySelector("img") !== null) return;
        const placeholder = element.querySelector(":scope > svg");
        if (placeholder !== null) placeholder.replaceWith(image);
        else element.prepend(image);
      };
      // Leave the placeholder intact if the file cannot be read or decoded.
      image.src = source;
    }
  }

  function load(next: readonly (string | null)[]): void {
    paths = next;
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => setTimeout(() => {
      scheduled = false;
      for (const path of new Set(paths)) {
        // Rejected image references leave the page usable; a later render retries.
        void avatars.source(path).then(source => display(path, source), () => {});
      }
    }, 0));
  }
  return { load };
}
