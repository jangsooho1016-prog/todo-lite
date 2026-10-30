<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { acceleratorFromEvent } from '../lib/accelerator';
  import { api } from '../lib/api';
  import { errorText } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { ShortcutStatus } from '../lib/types';

  let { onclose }: { onclose: () => void } = $props();

  const DEFAULT_SHORTCUT = 'Ctrl+Alt+Space';

  let shortcut = $state<ShortcutStatus | null>(null);
  let shortcutError = $state<string | null>(null);
  let recording = $state(false);
  let autostartOn = $state(false);
  let autostartBusy = $state(false);
  let autostartError = $state<string | null>(null);

  onMount(() => {
    api.quickShortcut().then(
      (s) => (shortcut = s),
      (e) => (shortcutError = errorText(e)),
    );
    api.autostart().then(
      (on) => (autostartOn = on),
      (e) => (autostartError = errorText(e)),
    );
  });

  // 조합을 기록하는 동안에만 window의 capture 단계에서 키를 가로챈다. App의 Esc(설정 닫기) 핸들러는
  // bubble 단계라서, 여기서 stopPropagation 하면 기록 취소만 되고 대화상자는 닫히지 않는다.
  $effect(() => {
    if (!recording) return;
    const capture = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === 'Escape') {
        recording = false;
        return;
      }
      const value = acceleratorFromEvent(e);
      if (value) {
        recording = false;
        void applyShortcut(value);
      }
    };
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  });

  async function applyShortcut(value: string) {
    shortcutError = null;
    try {
      shortcut = await api.setQuickShortcut(value);
    } catch (e) {
      shortcutError = errorText(e); // 이전 조합은 그대로 둔다
    }
  }

  async function toggleAutostart(checked: boolean) {
    autostartBusy = true;
    autostartError = null;
    try {
      autostartOn = await api.setAutostart(checked);
    } catch (e) {
      autostartError = errorText(e);
      autostartOn = !checked;
    } finally {
      autostartBusy = false;
    }
  }

  async function pick() {
    const dir = await open({ directory: true, multiple: false, title: '동기화 폴더 선택' });
    if (typeof dir === 'string') await app.setSyncFolder(dir);
  }
</script>

<button class="backdrop" aria-label="닫기" onclick={onclose}></button>
<div class="dialog settings" role="dialog" aria-modal="true" aria-label="설정">
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

  <section class="setting">
    <h3>빠른 입력 단축키</h3>
    <p class="hint">다른 프로그램을 쓰는 중에도 이 조합을 누르면 작은 입력창이 떠요.</p>
    <div class="folder-line shortcut-line">
      {#if recording}
        <span class="recording">원하는 조합을 누르세요… (Esc 취소)</span>
      {:else if shortcut}
        <kbd>{shortcut.current}</kbd>
      {:else}
        <span class="hint">불러오는 중…</span>
      {/if}
    </div>
    {#if shortcut?.error}
      <p class="error-line">등록되지 않았어요: {shortcut.error}</p>
      <p class="hint">트레이 메뉴의 “빠른 추가”는 그대로 쓸 수 있어요.</p>
    {/if}
    {#if shortcutError}<p class="error-line">{shortcutError}</p>{/if}
    <div class="dialog-actions">
      <button onclick={() => (recording = true)} disabled={recording}>변경</button>
      <button onclick={() => applyShortcut(DEFAULT_SHORTCUT)} disabled={recording}>기본값({DEFAULT_SHORTCUT})</button>
    </div>
  </section>

  <section class="setting">
    <h3>Windows 시작 시 자동 실행</h3>
    <label class="check-line">
      <input
        type="checkbox"
        checked={autostartOn}
        disabled={autostartBusy}
        onchange={(e) => toggleAutostart(e.currentTarget.checked)}
      />
      <span>Windows 시작 시 자동 실행</span>
    </label>
    <p class="hint">켜면 로그인할 때 창 없이 트레이로 시작해요. 설치한 앱에서 켜 주세요.</p>
    {#if autostartError}<p class="error-line">{autostartError}</p>{/if}
  </section>

  <div class="dialog-footer">
    <button class="primary" onclick={onclose}>닫기</button>
  </div>
</div>
