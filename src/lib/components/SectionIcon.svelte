<script lang="ts">
  import SfSymbol from "$lib/components/SfSymbol.svelte";
  import { SECTION_ICON_SYMBOLS, type SectionIconName } from "$lib/sf-symbols/section-icons";

  export type { SectionIconName };

  const {
    name,
    class: className = "form-title-icon",
  }: {
    name: SectionIconName;
    class?: string;
  } = $props();
</script>

<SfSymbol name={SECTION_ICON_SYMBOLS[name]} class={className} />
