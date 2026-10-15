import { requireElement } from "../../shared/dom";

/** Displays the actual slider value using the native memory editor's precision. */
export function updateScoreOutput(element: HTMLInputElement): void {
  const field = requireElement(element.closest(".score-field"), HTMLElement);
  requireElement(field.querySelector("output"), HTMLOutputElement).textContent = Number(element.value).toFixed(2);
}
