<script lang="ts">
  import { draftFromTask, draftIsDirty, editFromDraft, type Draft } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { Priority, Task } from '../lib/types';

  let { task }: { task: Task } = $props();

  /* svelte-ignore state_referenced_locally */
  const initial = draftFromTask(task);
  // 선택된 할 일이 바뀌면 이 컴포넌트가 새로 만들어지므로 초기값만 복사한다.
  let title = $state(initial.title);
  let dueDate = $state(initial.dueDate);
  let dueTime = $state(initial.dueTime);
  let priority = $state<Priority>(initial.priority);
  let tagText = $state(initial.tagText);
  let notes = $state(initial.notes);

  let panel: HTMLDivElement | undefined = $state();
  /* svelte-ignore state_referenced_locally */
  let lastSeen = task.updated_at;

  // 밖에서(동기화 등) 할 일이 바뀌었고 사용자가 이 패널을 편집 중이 아니면 입력칸을 새로 채운다.
  $effect(() => {
    const stamp = task.updated_at;
    if (stamp === lastSeen) return;
    lastSeen = stamp;
    if (panel?.contains(document.activeElement)) return;
    const d = draftFromTask(task);
    title = d.title;
    notes = d.notes;
    dueDate = d.dueDate;
    dueTime = d.dueTime;
    priority = d.priority;
    tagText = d.tagText;
  });

  async function save() {
    const draft: Draft = { title, notes, dueDate, dueTime, priority, tagText };
    if (!draftIsDirty(task, draft)) return;
    const ok = await app.save(task.id, editFromDraft(draft));
    if (!ok) title = task.title; // 제목을 비웠다면 원래 제목으로 되돌린다
  }

  function commitOnEnter(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.isComposing) (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<div class="detail-panel" bind:this={panel}>
  <label class="field">
    <span>제목</span>
    <input id="detail-title" type="text" bind:value={title} onblur={save} onkeydown={commitOnEnter} />
  </label>

  <div class="field-row">
    <label class="field">
      <span>마감일</span>
      <input type="date" bind:value={dueDate} onchange={save} />
    </label>
    <label class="field">
      <span>시각</span>
      <input type="time" bind:value={dueTime} onchange={save} />
    </label>
  </div>

  <label class="field">
    <span>우선순위</span>
    <select bind:value={priority} onchange={save}>
      <option value="high">높음</option>
      <option value="normal">보통</option>
      <option value="low">낮음</option>
    </select>
  </label>

  <label class="field">
    <span>태그</span>
    <input type="text" placeholder="#학교 #개인" bind:value={tagText} onblur={save} />
  </label>

  <label class="field grow">
    <span>메모</span>
    <textarea bind:value={notes} onblur={save}></textarea>
  </label>

  <div class="detail-actions">
    <button onclick={() => app.toggle(task.id)}>{task.completed_at ? '완료 취소' : '완료로 표시'}</button>
    <button class="danger" onclick={() => app.remove(task.id)}>삭제</button>
  </div>
</div>
