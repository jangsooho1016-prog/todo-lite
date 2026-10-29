<script lang="ts">
  import { app, VIEWS } from '../lib/state.svelte';

  let { onsettings }: { onsettings: () => void } = $props();
</script>

<nav class="sidebar" aria-label="보기">
  <ul class="nav-list">
    {#each VIEWS as v (v.name)}
      <li>
        <button
          class="nav-item"
          class:active={app.view === v.name && !app.tag && !app.searching}
          onclick={() => app.setView(v.name)}
        >
          {v.label}
        </button>
      </li>
    {/each}
  </ul>

  {#if app.tags.length > 0}
    <div class="nav-title">태그</div>
    <ul class="nav-list">
      {#each app.tags as t (t.name)}
        <li>
          <button class="nav-item" class:active={app.tag === t.name} onclick={() => app.setTag(t.name)}>
            <span>#{t.name}</span>
            <span class="count">{t.count}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="spacer"></div>
  <button class="nav-item" onclick={onsettings}>
    설정{#if app.syncFolder}<span class="dot" title="동기화 사용 중"></span>{/if}
  </button>
</nav>
