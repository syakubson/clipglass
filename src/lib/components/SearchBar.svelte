<script lang="ts">
  import SfSymbol from "$lib/components/SfSymbol.svelte";

  const {
    value = "",
    onchange,
  }: {
    value?: string;
    onchange?: (v: string) => void;
  } = $props();

  let inputEl: HTMLInputElement | undefined = $state();
  let focused = $state(false);

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    onchange?.(target.value);
  }

  function handleClear() {
    onchange?.("");
    inputEl?.focus();
  }

  export function focus() {
    inputEl?.focus();
    inputEl?.select();
  }

  export function blur() {
    inputEl?.blur();
  }

  export function isFocused() {
    return focused;
  }
</script>

<div class="search-bar text-control-host" role="search">
  <SfSymbol name="magnifyingglass" class="search-icon" />
  <input
    bind:this={inputEl}
    type="search"
    placeholder="Search clipboard history..."
    aria-label="Search clipboard history"
    autocomplete="off"
    spellcheck="false"
    {value}
    oninput={handleInput}
    onfocus={() => {
      focused = true;
    }}
    onblur={() => {
      focused = false;
    }}
  />
  <button
    type="button"
    class="clear-btn"
    class:hidden={!value}
    aria-label="Clear search"
    aria-hidden={!value}
    tabindex={value ? 0 : -1}
    disabled={!value}
    onclick={handleClear}
  >
    <SfSymbol name="xmark" class="search-clear-icon" />
  </button>
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    height: var(--overlay-header-control-height);
    padding: 0 10px 0 12px;
    border-radius: var(--radius-control);
    width: 280px;
    flex: 0 0 280px;
    background-color: var(--surface-search);
  }

  .search-bar:hover:not(:focus-within) {
    background-color: var(--surface-search-hover);
  }

  .search-bar:focus-within {
    background-color: var(--surface-search-focus);
  }

  .search-bar :global(.search-icon) {
    color: var(--color-search-icon);
    flex-shrink: 0;
  }

  input {
    background: none;
    border: none;
    outline: none;
    color: var(--color-search-input);
    font-size: var(--font-size-md);
    width: 100%;
    min-width: 0;
    font-family: inherit;
    user-select: text;
  }

  input::placeholder {
    color: var(--color-search-placeholder);
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    /* 28px hit target (HIG); layout footprint ~20px via negative margin */
    width: 28px;
    height: 28px;
    margin: -4px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--duration-fast) var(--ease-interactive);
  }

  /* 20px visual circle — separate from the 28px click target */
  .clear-btn::before {
    content: "";
    position: absolute;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-10);
    pointer-events: none;
    transition: background var(--duration-fast) var(--ease-interactive);
  }

  .clear-btn:hover {
    color: var(--color-text-secondary);
  }

  .clear-btn:hover::before {
    background: var(--surface-12);
  }

  .clear-btn :global(svg) {
    position: relative;
    z-index: 1;
  }

  .clear-btn:focus-visible {
    outline: none;
    box-shadow: var(--ring-accent);
  }

  .clear-btn.hidden {
    visibility: hidden;
    pointer-events: none;
  }
</style>
