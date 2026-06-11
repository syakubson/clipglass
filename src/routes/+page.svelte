<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { ClipboardEntry, Collection } from "$lib/types";
  import {
    getEntries,
    getCollections,
    hideMainWindow,
    openSettingsWindow,
    activateEntry,
    isTaggingReady,
    getExcludableAppCandidate,
    addExcludableAppCandidate,
  } from "$lib/api";
  import type { ExcludableAppCandidate } from "$lib/types";
  import {
    alreadyExcludedFromHistoryLabel,
    excludeFromClipboardHistoryAriaLabel,
    excludeFromHistoryLabel,
    invokeErrorMessage,
  } from "$lib/exclusion-label";
  import ClipboardCard from "$lib/components/ClipboardCard.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import CollectionTabs from "$lib/components/CollectionTabs.svelte";
  import { overlayEscapeAction } from "$lib/overlay-search";
  import { panelCloseFallbackMs, panelOpenMs, scrollBehavior } from "$lib/motion";

  let entries: ClipboardEntry[] = $state([]);
  let collections: Collection[] = $state([]);
  let searchQuery = $state("");
  let activeCollectionId: number | null = $state(null);
  let pinnedOnly = $state(false);
  let activeTag = $state<string | null>(null);
  let selectedIndex = $state(-1);
  let gridEl: HTMLDivElement | undefined = $state();
  let appEl: HTMLDivElement | undefined = $state();
  let visible = $state(false);
  let isRevealing = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let revealTimer: ReturnType<typeof setTimeout> | undefined;
  let pendingReload = false;
  let revealSeq = 0;
  let hideTransitionHandler: ((e: TransitionEvent) => void) | undefined;
  let retagAvailable = $state(false);
  let excludeCandidate: ExcludableAppCandidate | null = $state(null);
  let excludeNotice = $state("");
  let excludeNoticeTone = $state<"neutral" | "warn">("neutral");
  let excludeBusy = $state(false);
  let searchBar: SearchBar | undefined = $state();
  const hiddenTopTags = new Set(["code", "otp", "token", "log"]);
  const imageFormatTags = ["gif", "jpg", "png"];
  const imageFormatTagSet = new Set(imageFormatTags);

  async function syncRetagAvailability() {
    retagAvailable = await isTaggingReady();
  }

  async function loadExcludeCandidate() {
    try {
      const candidate = await getExcludableAppCandidate();
      excludeCandidate = candidate;
      if (candidate?.alreadyExcluded) {
        excludeNotice = alreadyExcludedFromHistoryLabel(candidate.displayName);
        excludeNoticeTone = "neutral";
        return;
      }
      excludeNotice = "";
    } catch (err) {
      excludeCandidate = null;
      excludeNotice = invokeErrorMessage(err) || "Could not detect active app";
      excludeNoticeTone = "warn";
    }
  }

  async function handleExcludeFromPanel() {
    if (excludeBusy) return;
    excludeBusy = true;
    try {
      const added = await addExcludableAppCandidate();
      if (added) {
        await loadExcludeCandidate();
        return;
      }
      excludeNotice = "No active app";
      excludeNoticeTone = "warn";
    } catch (err) {
      excludeNotice = invokeErrorMessage(err) || "Could not exclude this app";
      excludeNoticeTone = "warn";
    } finally {
      excludeBusy = false;
    }
  }

  async function loadEntries(selectFirst = false, scrollToFirst = true) {
    entries = await getEntries({
      collection_id: activeCollectionId,
      pinned_only: pinnedOnly,
      search: searchQuery || null,
    });
    if (selectFirst) {
      selectedIndex = filteredEntries.length > 0 ? 0 : -1;
      if (scrollToFirst) scrollToSelected();
    }
  }

  function nextPaint(): Promise<void> {
    return new Promise((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
    });
  }

  function clearHideTimer() {
    if (hideTimer !== undefined) {
      clearTimeout(hideTimer);
      hideTimer = undefined;
    }
  }

  function clearRevealTimer() {
    if (revealTimer !== undefined) {
      clearTimeout(revealTimer);
      revealTimer = undefined;
    }
  }

  function clearHideTransitionHandler() {
    if (hideTransitionHandler && appEl) {
      appEl.removeEventListener("transitionend", hideTransitionHandler);
      hideTransitionHandler = undefined;
    }
  }

  function requestNativeHide() {
    clearHideTimer();
    clearHideTransitionHandler();

    let committed = false;
    const commit = () => {
      if (committed) return;
      committed = true;
      clearHideTimer();
      clearHideTransitionHandler();
      void hideMainWindow();
    };

    const onTransitionEnd = (e: TransitionEvent) => {
      if (e.target !== appEl || e.propertyName !== "opacity") return;
      commit();
    };

    hideTransitionHandler = onTransitionEnd;
    appEl?.addEventListener("transitionend", onTransitionEnd);
    hideTimer = setTimeout(() => {
      hideTimer = undefined;
      commit();
    }, panelCloseFallbackMs());
  }

  async function loadCollections() {
    collections = await getCollections();
  }

  function finishReveal() {
    isRevealing = false;
    revealTimer = undefined;
    if (pendingReload) {
      pendingReload = false;
      void loadEntries(true, false);
    }
  }

  function resetOverlayMotionState() {
    revealSeq += 1;
    clearRevealTimer();
    isRevealing = false;
    visible = false;
    clearSearch({ reload: false });
    activeTag = null;
    selectedIndex = -1;
  }

  function showWindow() {
    const seq = ++revealSeq;
    window.getSelection()?.removeAllRanges();
    clearHideTimer();
    clearHideTransitionHandler();
    clearRevealTimer();
    clearSearch({ reload: false });
    activeTag = null;

    isRevealing = true;
    pendingReload = false;
    if (gridEl) gridEl.scrollLeft = 0;

    // Always reset to hidden first so CSS transition replays on every open.
    visible = false;
    void nextPaint().then(() => {
      if (seq !== revealSeq) return;
      visible = true;
      searchBar?.blur();
      void loadEntries(true, false);
      revealTimer = setTimeout(finishReveal, panelOpenMs());
      void syncRetagAvailability();
      void loadExcludeCandidate();
    });
  }

  function startVisualHide() {
    revealSeq += 1;
    clearRevealTimer();
    isRevealing = false;
    pendingReload = false;
    visible = false;
  }

  function animateOut() {
    startVisualHide();
    requestNativeHide();
  }

  function forceHideWindow() {
    animateOut();
  }

  onMount(() => {
    void syncRetagAvailability();
    loadEntries();
    loadCollections();

    // Tell Rust we're loaded — it will hide the off-screen warmup window
    invoke("frontend_ready");

    // Debounce entry reloads — clipboard-changed and entry-tagged can fire together
    let reloadTimer: ReturnType<typeof setTimeout>;
    function scheduleReload() {
      if (isRevealing) {
        pendingReload = true;
        return;
      }
      clearTimeout(reloadTimer);
      reloadTimer = setTimeout(() => loadEntries(), 100);
    }

    const unlistenClipboard = listen("clipboard-changed", scheduleReload);
    const unlistenTagged = listen("entry-tagged", scheduleReload);

    const unlistenShow = listen("window-show", () => {
      showWindow();
    });

    const unlistenHideRequest = listen("window-hide-request", () => {
      startVisualHide();
      requestNativeHide();
    });

    const unlistenHide = listen("window-hide", () => {
      clearHideTimer();
      clearHideTransitionHandler();
      resetOverlayMotionState();
    });

    const unlistenOpenSettings = listen("open-settings", () => {
      openSettingsWindow();
    });

    const handleKeydown = (e: KeyboardEvent) => {
      if (!visible) return;

      const searchFocused = searchBar?.isFocused() ?? false;
      const target = e.target;
      const typingInField =
        target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (overlayEscapeAction(searchQuery.length > 0) === "clear-search") {
          clearSearch({ immediate: true });
          searchBar?.blur();
          return;
        }
        forceHideWindow();
        return;
      }

      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        e.stopPropagation();
        searchBar?.focus();
        return;
      }

      if (
        e.key === "/" &&
        !searchFocused &&
        !typingInField &&
        !e.metaKey &&
        !e.ctrlKey &&
        !e.altKey
      ) {
        e.preventDefault();
        searchBar?.focus();
        return;
      }

      if (e.key === "ArrowRight") {
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, filteredEntries.length - 1);
        scrollToSelected();
        return;
      }

      if (e.key === "ArrowLeft") {
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        scrollToSelected();
        return;
      }

      if (e.key === "Enter" && selectedIndex >= 0 && selectedIndex < filteredEntries.length) {
        e.preventDefault();
        const entry = filteredEntries[selectedIndex];
        if (entry.content_type === "text" || entry.content_type === "image") {
          void activateEntry(entry.id);
        }
      }
    };

    window.addEventListener("keydown", handleKeydown, true);

    return () => {
      clearHideTimer();
      clearHideTransitionHandler();
      clearRevealTimer();
      clearTimeout(reloadTimer);
      clearTimeout(debounceTimer);
      unlistenClipboard.then((fn) => fn());
      unlistenTagged.then((fn) => fn());
      unlistenShow.then((fn) => fn());
      unlistenHideRequest.then((fn) => fn());
      unlistenHide.then((fn) => fn());
      unlistenOpenSettings.then((fn) => fn());
      window.removeEventListener("keydown", handleKeydown, true);
    };
  });

  function scrollToSelected() {
    if (!gridEl) return;
    const cards = gridEl.querySelectorAll(".card");
    if (cards[selectedIndex]) {
      cards[selectedIndex].scrollIntoView({
        behavior: scrollBehavior(),
        block: "nearest",
        inline: "center",
      });
    }
  }

  function setSearchQuery(
    q: string,
    options: { reload?: boolean; immediate?: boolean } = {},
  ) {
    const { reload = true, immediate = false } = options;
    searchQuery = q;
    clearTimeout(debounceTimer);
    if (!reload) return;
    if (immediate || q === "") {
      void loadEntries(true);
      return;
    }
    debounceTimer = setTimeout(() => void loadEntries(true), 150);
  }

  function queueSearch(q: string) {
    setSearchQuery(q);
  }

  function clearSearch(options: { reload?: boolean; immediate?: boolean } = {}) {
    setSearchQuery("", options);
  }

  function handleCollectionSelect(id: number | null) {
    pinnedOnly = id === -1;
    activeCollectionId = id === -1 ? null : id;
    activeTag = null;
    void loadEntries(true);
  }

  function handleEntryAction() {
    loadEntries();
  }

  let debounceTimer: ReturnType<typeof setTimeout>;
  function debouncedSearch(q: string) {
    if (q === "") {
      clearSearch({ immediate: true });
      return;
    }
    queueSearch(q);
  }

  function emptyStateCopy(): { title: string; hint?: string } {
    if (searchQuery && activeTag) {
      return {
        title: `No results for “${searchQuery}” in tag “${activeTag}”`,
        hint: "Try a different search or tag",
      };
    }
    if (searchQuery) {
      return {
        title: `No results for “${searchQuery}”`,
        hint: "Try a different search term",
      };
    }
    if (activeTag) {
      return {
        title: `No results for tag “${activeTag}”`,
        hint: "Try another tag or clear the filter",
      };
    }
    return {
      title: "Clipboard history is empty",
      hint: "Copy something to get started",
    };
  }

  function sortTagsByCount(tagCounts: [string, number][]) {
    return tagCounts.sort((a, b) => {
      if (b[1] !== a[1]) return b[1] - a[1];
      return a[0].localeCompare(b[0]);
    });
  }

  let topTags = $derived.by(() => {
    const counts = new Map<string, number>();

    for (const entry of entries) {
      for (const tag of entry.tags ?? []) {
        if (hiddenTopTags.has(tag)) continue;
        counts.set(tag, (counts.get(tag) ?? 0) + 1);
      }
    }

    const pinnedFormatTags = sortTagsByCount(
      imageFormatTags
        .filter((tag) => counts.has(tag))
        .map((tag) => [tag, counts.get(tag)!] as [string, number]),
    );

    const contentTags = sortTagsByCount(
      [...counts.entries()].filter(([tag]) => !imageFormatTagSet.has(tag)),
    ).slice(0, 8);

    return [...pinnedFormatTags, ...contentTags];
  });

  function entryMatchesTag(entry: ClipboardEntry, tag: string): boolean {
    if ((entry.tags ?? []).includes(tag)) return true;
    if (!imageFormatTagSet.has(tag) || entry.content_type !== "image") return false;
    return entry.image_format?.toLowerCase() === tag;
  }

  let filteredEntries = $derived.by(() => {
    if (!activeTag) return entries;
    const tag = activeTag;
    return entries.filter((entry) => entryMatchesTag(entry, tag));
  });

  function resetKeyboardSelection() {
    selectedIndex = filteredEntries.length > 0 ? 0 : -1;
    scrollToSelected();
  }
</script>

<div class="app" class:visible bind:this={appEl}>
  <header class="header">
    <SearchBar bind:this={searchBar} value={searchQuery} onchange={debouncedSearch} />
    <CollectionTabs
      {collections}
      activeId={activeCollectionId}
      activePinned={pinnedOnly}
      onselect={handleCollectionSelect}
      onupdate={loadCollections}
    />
    <div class="header-actions">
      {#if excludeCandidate && !excludeCandidate.alreadyExcluded}
        {@const excludeLabel = excludeFromClipboardHistoryAriaLabel(
          excludeCandidate.displayName,
        )}
        <button
          class="form-btn-restrict exclude-app-btn app-btn"
          type="button"
          aria-label={excludeLabel}
          title={excludeLabel}
          aria-busy={excludeBusy}
          disabled={excludeBusy}
          onclick={() => void handleExcludeFromPanel()}
        >
          <span class="exclude-app-btn-text"
            >{excludeFromHistoryLabel(excludeCandidate.displayName)}</span
          >
        </button>
      {/if}
      {#if excludeNotice}
        <span
          class="status-hint exclude-notice"
          class:neutral={excludeNoticeTone === "neutral"}
          class:warn={excludeNoticeTone === "warn"}
          aria-live="polite"
        >
          {excludeNotice}
        </span>
      {/if}
      <button
        class="settings-btn app-btn"
        type="button"
        aria-label="Open settings"
        onclick={() => openSettingsWindow()}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M19.14 12.94c.04-.31.06-.62.06-.94s-.02-.63-.06-.94l2.03-1.58a.5.5 0 0 0 .12-.64l-1.92-3.32a.5.5 0 0 0-.6-.22l-2.39.96a7.03 7.03 0 0 0-1.63-.94l-.36-2.54a.5.5 0 0 0-.5-.42h-3.84a.5.5 0 0 0-.5.42l-.36 2.54c-.58.22-1.13.53-1.63.94l-2.39-.96a.5.5 0 0 0-.6.22L2.71 8.84a.5.5 0 0 0 .12.64l2.03 1.58c-.04.31-.06.62-.06.94s.02.63.06.94l-2.03 1.58a.5.5 0 0 0-.12.64l1.92 3.32a.5.5 0 0 0 .6.22l2.39-.96c.5.41 1.05.72 1.63.94l.36 2.54a.5.5 0 0 0 .5.42h3.84a.5.5 0 0 0 .5-.42l.36-2.54c.58-.22 1.13-.53 1.63-.94l2.39.96a.5.5 0 0 0 .6-.22l1.92-3.32a.5.5 0 0 0-.12-.64zM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7z"
          />
        </svg>
      </button>
    </div>
  </header>

  {#if topTags.length > 0}
    <div class="tag-groups">
      <button
        class="tag-group-chip app-btn"
        class:active={!activeTag}
        type="button"
        onclick={() => {
          activeTag = null;
          resetKeyboardSelection();
        }}
      >
        All tags
      </button>

      {#each topTags as [tag, count]}
        <button
          class="tag-group-chip app-btn"
          class:active={activeTag === tag}
          type="button"
          onclick={() => {
            activeTag = tag;
            resetKeyboardSelection();
          }}
        >
          <span>{tag}</span>
          <span class="tag-group-count">{count}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="grid-container" bind:this={gridEl}>
    {#if filteredEntries.length === 0}
      {@const empty = emptyStateCopy()}
      <div class="empty-state" role="status" aria-live="polite">
        <p class="empty-title">{empty.title}</p>
        {#if empty.hint}
          <p class="hint">{empty.hint}</p>
        {/if}
      </div>
    {:else}
      {#each filteredEntries as entry, i (entry.id)}
        <div class="card-wrapper">
          <ClipboardCard
            {entry}
            {retagAvailable}
            selected={i === selectedIndex}
            ondeleted={handleEntryAction}
            onpinned={handleEntryAction}
            onretagged={handleEntryAction}
          />
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
    color: var(--color-text-body);
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
  }

  :global(*) {
    box-sizing: border-box;
    outline: none;
  }

  .app {
    width: 100vw;
    height: 100vh;
    background: var(--surface-app);
    backdrop-filter: blur(var(--panel-blur-visible)) saturate(1.15);
    -webkit-backdrop-filter: blur(var(--panel-blur-visible)) saturate(1.15);
    border-radius: 18px;
    border: 1px solid var(--border-strong);
    box-shadow:
      var(--shadow-elevated),
      var(--shadow-inset-highlight);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    backface-visibility: hidden;
    transform: translate3d(0, var(--panel-open-travel), 0);
    opacity: 0;
    will-change: transform, opacity;
    /* Transition on hidden state runs when opening (visible added). */
    transition:
      transform var(--duration-panel-open) var(--ease-apple-panel),
      opacity var(--duration-panel-opacity-open) var(--ease-apple-panel);
  }

  .app.visible {
    transform: translate3d(0, 0, 0);
    opacity: 1;
    will-change: auto;
    /* Transition on visible state runs when closing (visible removed). */
    transition:
      transform var(--duration-panel-close) var(--ease-panel-dismiss),
      opacity var(--duration-panel-opacity-close) var(--ease-panel-dismiss);
  }

  @media (prefers-reduced-motion: reduce) {
    .app,
    .app.visible {
      transition-duration: 0.01ms;
    }
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-default);
    background: var(--surface-1);
    flex-shrink: 0;
  }

  .tag-groups {
    display: flex;
    gap: 8px;
    padding: 10px 16px 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tag-groups::-webkit-scrollbar {
    display: none;
  }

  .tag-group-chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 11px;
    border-radius: 999px;
    border: 1px solid var(--border-soft);
    background: var(--surface-3);
    color: var(--color-text-secondary);
    cursor: pointer;
    white-space: nowrap;
    font: inherit;
    font-size: 11px;
    transition:
      background var(--duration-fast) var(--ease-interactive),
      border-color var(--duration-fast) var(--ease-interactive),
      color var(--duration-fast) var(--ease-interactive);
  }

  .tag-group-chip:hover:not(:disabled):not([aria-busy="true"]) {
    background: var(--surface-7);
    border-color: var(--border-strong);
  }

  .tag-group-chip.active {
    background: var(--surface-accent);
    border-color: var(--border-accent-soft);
    color: var(--color-accent-chip);
  }

  .tag-group-count {
    display: inline-flex;
    min-width: 18px;
    justify-content: center;
    padding: 2px 5px;
    border-radius: 999px;
    background: var(--surface-8);
    font-size: 10px;
    line-height: 1;
  }

  .header-actions {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .exclude-app-btn {
    height: 36px;
    max-width: min(220px, 42vw);
    padding: 0 12px;
    border-radius: 10px;
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .exclude-app-btn-text {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .exclude-notice {
    margin: 0;
    white-space: nowrap;
  }

  .settings-btn {
    width: 36px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-6);
    border: 1px solid var(--border-soft);
    border-radius: 10px;
    color: var(--color-text-body);
    cursor: pointer;
  }

  .settings-btn:hover:not(:disabled):not([aria-busy="true"]) {
    background: var(--surface-10);
    border-color: var(--border-emphasis);
  }

  .settings-btn svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }

  .grid-container {
    flex: 1;
    display: flex;
    gap: 12px;
    padding: 14px 16px var(--space-section);
    overflow-x: auto;
    overflow-y: hidden;
    align-items: flex-start;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
    min-height: 0;
  }

  .grid-container::-webkit-scrollbar {
    height: 6px;
  }

  .grid-container::-webkit-scrollbar-track {
    background: transparent;
  }

  .grid-container::-webkit-scrollbar-thumb {
    background: var(--scrollbar-thumb);
    border-radius: 3px;
  }

  .card-wrapper {
    flex-shrink: 0;
  }

  .empty-state {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 0 24px;
    text-align: center;
    color: var(--color-text-tertiary);
  }

  .empty-title {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--color-text-secondary);
  }

  .hint {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--color-text-label);
  }
</style>
