<script lang="ts">
  import { api } from '../lib/api';
  import { formatDue, todayIso } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { ParsedTask } from '../lib/types';

  let text = $state('');
  let parsed = $state<ParsedTask | null>(null);
  let seq = 0;

  const due = $derived(parsed ? formatDue(parsed.due_date, parsed.due_time, todayIso()) : '');

  async function onInput() {
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

  async function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    const task = await app.addFromInput(text);
    if (task) {
      text = '';
      parsed = null;
      seq++;
    }
  }
</script>

<div class="quick-add">
  <input
    id="quick-add-input"
    type="text"
    placeholder="예: 내일 오후 3시 팀플 회의 #학교 !중요   (Ctrl+N)"
    autocomplete="off"
    bind:value={text}
    oninput={onInput}
    onkeydown={onKeydown}
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
