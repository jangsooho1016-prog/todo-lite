<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { errorText } from '../lib/format';
  import QuickAdd from './QuickAdd.svelte';

  let error = $state<string | null>(null);
  let quickAdd: QuickAdd | undefined;

  onMount(() => {
    // 백엔드는 창을 보여 줄 때마다 이 이벤트를 보낸다. 처음 만든 창은 놓칠 수 있어 QuickAdd가 마운트 때도 포커스한다.
    const unlisten = listen('quick-opened', () => {
      error = null;
      quickAdd?.reset();
    });
    return () => void unlisten.then((fn) => fn());
  });

  async function submit(text: string): Promise<boolean> {
    try {
      await api.addTask(text);
      await api.hideQuick();
      return true;
    } catch (e) {
      error = errorText(e);
      return false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.isComposing) return;
    e.preventDefault();
    void api.hideQuick();
  }
</script>

<svelte:window {onkeydown} />

<div class="quick-window">
  <div class="quick-head">
    <span class="quick-title">빠른 추가</span>
    <span class="quick-hint">Enter 저장 · Esc 닫기</span>
  </div>
  <QuickAdd bind:this={quickAdd} {submit} inputId="quick-window-input" autofocus oninput={() => (error = null)} />
  {#if error}<p class="quick-error" role="alert">{error}</p>{/if}
</div>
