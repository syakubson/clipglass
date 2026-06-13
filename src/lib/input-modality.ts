/**
 * WebKit in Tauri often matches :focus-visible on mouse click for text fields.
 * Track last input modality so focus rings appear only after keyboard navigation.
 */
export function initInputModality(root: HTMLElement = document.documentElement): () => void {
  const setModality = (modality: "pointer" | "keyboard") => {
    root.dataset.inputModality = modality;
  };

  const onPointerDown = () => setModality("pointer");

  const onKeyDown = (e: KeyboardEvent) => {
    const target = e.target;
    const typingInField =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement;

    if (e.key === "Tab") {
      setModality("keyboard");
      return;
    }

    if (e.key.startsWith("Arrow") || e.key === "Enter") {
      if (!typingInField) setModality("keyboard");
      return;
    }

    // Panel shortcuts (⌘F, paste, etc.) outside a text field still deserve the keyboard ring.
    if ((e.metaKey || e.ctrlKey || e.altKey) && !typingInField) {
      setModality("keyboard");
      return;
    }

    if (e.key === "/" && !typingInField) {
      setModality("keyboard");
    }
  };

  document.addEventListener("pointerdown", onPointerDown, true);
  document.addEventListener("keydown", onKeyDown, true);

  return () => {
    document.removeEventListener("pointerdown", onPointerDown, true);
    document.removeEventListener("keydown", onKeyDown, true);
    delete root.dataset.inputModality;
  };
}
