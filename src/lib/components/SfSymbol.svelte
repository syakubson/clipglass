<script lang="ts">
  import { SF_SYMBOL_PATHS, SF_SYMBOL_VIEWBOX, type SfSymbolName } from "$lib/sf-symbols";

  const {
    name,
    class: className = "",
    title,
  }: {
    name: SfSymbolName;
    class?: string;
    title?: string;
  } = $props();
</script>

<!-- width/height attrs prevent WebKit default ~300×150 when CSS has not loaded yet -->
<svg
  class="sf-symbol {className}"
  width={SF_SYMBOL_VIEWBOX}
  height={SF_SYMBOL_VIEWBOX}
  viewBox="0 0 {SF_SYMBOL_VIEWBOX} {SF_SYMBOL_VIEWBOX}"
  fill="currentColor"
  aria-hidden={title ? undefined : true}
  role={title ? "img" : undefined}
>
  {#if title}
    <title>{title}</title>
  {/if}
  <path d={SF_SYMBOL_PATHS[name]} />
</svg>
