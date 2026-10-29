<script lang="ts">
  import { onMount } from 'svelte';
  import DetailPanel from './components/DetailPanel.svelte';
  import QuickAdd from './components/QuickAdd.svelte';
  import Settings from './components/Settings.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TaskList from './components/TaskList.svelte';
  import { actionForKey, targetKind } from './lib/keys';
  import { app } from './lib/state.svelte';

  let settingsOpen = $state(false);

  onMount(() => {
    void app.init();
  });

  function focusById(id: string) {
    const el = document.getElementById(id) as HTMLInputElement | null;
    el?.focus();
    el?.select?.();
  }

  function onkeydown(e: KeyboardEvent) {
    if (settingsOpen) {
      if (e.key === 'Escape') settingsOpen = false;
      return;
    }
    const action = actionForKey(
      {
        key: e.key,
        ctrlKey: e.ctrlKey || e.metaKey,
        isComposing: e.isComposing,
        code: e.code,
        keyCode: (e as KeyboardEvent & { keyCode?: number }).keyCode,
      },
      targetKind(e.target),
    );
    if (!action) return;
    e.preventDefault();

    switch (action.type) {
      case 'focus-add':
        focusById('quick-add-input');
        break;
      case 'focus-search':
        focusById('search-input');
        break;
      case 'move':
        app.moveSelection(action.delta);
        break;
      case 'toggle':
        app.toggleSelected();
        break;
      case 'edit':
        if (app.selected) focusById('detail-title');
        break;
      case 'delete':
        app.removeSelected();
        break;
      case 'escape': {
        const active = document.activeElement as HTMLElement | null;
        if (active?.id === 'search-input' && app.search) app.clearSearch();
        active?.blur();
        break;
      }
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="app">
  <Sidebar onsettings={() => (settingsOpen = true)} />

  <main class="center">
    <header class="topbar">
      <QuickAdd />
      <input
        id="search-input"
        class="search"
        type="search"
        placeholder="검색 (Ctrl+F)"
        autocomplete="off"
        bind:value={app.search}
        oninput={() => app.onSearchInput()}
      />
    </header>
    <TaskList />
  </main>

  <aside class="detail" aria-label="상세">
    {#key app.selected?.id}
      {#if app.selected}
        <DetailPanel task={app.selected} />
      {:else}
        <p class="hint">할 일을 선택하면 자세한 내용이 여기에 보여요.</p>
      {/if}
    {/key}
  </aside>
</div>

{#if settingsOpen}
  <Settings onclose={() => (settingsOpen = false)} />
{/if}

{#if app.undo}
  <div class="toast" role="status">
    “{app.undo.title}” 완료
    <button onclick={() => app.undoComplete()}>되돌리기</button>
  </div>
{/if}
{#if app.error}
  <div class="toast error" role="alert">{app.error}</div>
{/if}
