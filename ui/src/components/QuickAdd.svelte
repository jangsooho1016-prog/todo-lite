<script lang="ts">
  import { api } from '../lib/api';
  import { formatDue, todayIso } from '../lib/format';
  import type { ParsedTask } from '../lib/types';
  import { onMount } from 'svelte';

  let {
    submit,
    inputId = 'quick-add-input',
    autofocus = false,
    oninput: onchange,
  }: {
    /** 저장을 시도한다. 저장됐으면 true. 실패하면 입력을 되돌린다. */
    submit: (text: string) => Promise<boolean>;
    inputId?: string;
    autofocus?: boolean;
    /** 사용자가 입력을 바꿀 때마다 불린다. */
    oninput?: () => void;
  } = $props();

  let inputEl = $state<HTMLInputElement | null>(null);

  onMount(() => {
    if (autofocus) inputEl?.focus();
  });

  /** 입력을 비우고 입력칸에 포커스를 준다. */
  export function reset() {
    seq++;
    text = '';
    parsed = null;
    inputEl?.focus();
  }

  let text = $state('');
  let parsed = $state<ParsedTask | null>(null);
  let seq = 0;

  const due = $derived(parsed ? formatDue(parsed.due_date, parsed.due_time, todayIso()) : '');

  async function onInput() {
    onchange?.();
    const mine = ++seq;
    if (!text.trim()) {
      parsed = null;
      return;
    }
    try {
      const result = await api.preview(text);
      if (mine === seq) parsed = result;
    } catch {
      // 미리보기 실패는 조용히 넘어간다. 저장할 때 오류가 표시된다.
    }
  }

  let busy = false;
  let composedAt = 0;

  async function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    // IME 조합 확정 Enter는 무시한다. 일부 WebView2는 isComposing=false에 keyCode 229로 보낸다.
    if (e.isComposing || e.keyCode === 229 || performance.now() - composedAt < 50) return;
    e.preventDefault();
    if (busy || !text.trim()) return;
    busy = true;
    const value = text;
    text = '';
    parsed = null;
    seq++;
    try {
      const saved = await submit(value);
      if (!saved && !text) {
        text = value; // 실패하면 입력을 잃지 않게 되돌린다
        void onInput();
      }
    } finally {
      busy = false;
    }
  }
</script>

<div class="quick-add">
  <input
    id={inputId}
    bind:this={inputEl}
    type="text"
    placeholder="예: 내일 3시 회의 #학교 !중요"
    title="Ctrl+N으로 바로 입력"
    autocomplete="off"
    bind:value={text}
    oninput={onInput}
    onkeydown={onKeydown}
    oncompositionend={() => (composedAt = performance.now())}
  />
  {#if parsed && text.trim()}
    <div class="chips" aria-live="polite">
      <span class="chip title">{parsed.title || '제목 없음'}</span>
      {#if due}<span class="chip due">마감 {due}</span>{/if}
      {#if parsed.priority === 'high'}<span class="chip high">중요</span>{/if}
      {#if parsed.priority === 'low'}<span class="chip low">낮음</span>{/if}
      {#each parsed.tags as tag (tag)}<span class="chip tag">#{tag}</span>{/each}
    </div>
  {/if}
</div>
