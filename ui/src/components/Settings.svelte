<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app } from '../lib/state.svelte';

  let { onclose }: { onclose: () => void } = $props();

  async function pick() {
    const dir = await open({ directory: true, multiple: false, title: '동기화 폴더 선택' });
    if (typeof dir === 'string') await app.setSyncFolder(dir);
  }
</script>

<button class="backdrop" aria-label="닫기" onclick={onclose}></button>
<div class="dialog" role="dialog" aria-modal="true" aria-label="설정">
  <h2>동기화 설정</h2>
  <p class="hint">
    Google Drive나 OneDrive처럼 자동으로 동기화되는 폴더를 고르면, 같은 폴더를 가리키는 다른 기기와 할 일이
    맞춰져요. 고르지 않으면 이 기기에만 저장돼요.
  </p>

  <div class="folder-line">
    <code>{app.syncFolder ?? '선택된 폴더 없음'}</code>
  </div>

  <div class="dialog-actions">
    <button onclick={pick}>폴더 선택…</button>
    <button onclick={() => app.setSyncFolder(null)} disabled={!app.syncFolder}>해제</button>
    <button onclick={() => app.runSync()} disabled={!app.syncFolder || app.syncing}>
      {app.syncing ? '동기화 중…' : '지금 동기화'}
    </button>
  </div>

  {#if app.syncMessage}<p class="sync-message">{app.syncMessage}</p>{/if}

  <div class="dialog-footer">
    <button class="primary" onclick={onclose}>닫기</button>
  </div>
</div>
