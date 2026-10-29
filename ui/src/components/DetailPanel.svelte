<script lang="ts">
  import { parseTagInput, tagsToText, timeInputValue } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { Priority, Task } from '../lib/types';

  let { task }: { task: Task } = $props();

  // 선택된 할 일이 바뀌면 이 컴포넌트가 새로 만들어지므로 초기값만 복사한다.
  /* svelte-ignore state_referenced_locally */
  let title = $state(task.title);
  /* svelte-ignore state_referenced_locally */
  let dueDate = $state(task.due_date ?? '');
  /* svelte-ignore state_referenced_locally */
  let dueTime = $state(timeInputValue(task.due_time));
  /* svelte-ignore state_referenced_locally */
  let priority = $state<Priority>(task.priority);
  /* svelte-ignore state_referenced_locally */
  let tagText = $state(tagsToText(task.tags));
  /* svelte-ignore state_referenced_locally */
  let notes = $state(task.notes ?? '');

  async function save() {
    const ok = await app.save(task.id, {
      title,
      notes: notes.trim() ? notes : null,
      due_date: dueDate || null,
      due_time: dueTime || null,
      priority,
      tags: parseTagInput(tagText),
    });
    if (!ok) title = task.title; // 제목을 비웠다면 원래 제목으로 되돌린다
  }
</script>

<div class="detail-panel">
  <label class="field">
    <span>제목</span>
    <input id="detail-title" type="text" bind:value={title} onblur={save} />
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
