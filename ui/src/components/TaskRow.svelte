<script lang="ts">
  import { formatDue, isOverdue, nowParts, todayIso } from '../lib/format';
  import type { Task } from '../lib/types';

  let {
    task,
    selected,
    onselect,
    ontoggle,
  }: { task: Task; selected: boolean; onselect: () => void; ontoggle: () => void } = $props();

  let el: HTMLLIElement | undefined = $state();

  const done = $derived(!!task.completed_at);
  const due = $derived(formatDue(task.due_date, task.due_time, todayIso()));
  const overdue = $derived(isOverdue(task, nowParts()));

  $effect(() => {
    if (selected) el?.scrollIntoView({ block: 'nearest' });
  });
</script>

<li class="row" class:selected class:done data-priority={task.priority} bind:this={el}>
  <input type="checkbox" checked={done} onchange={ontoggle} aria-label="완료" />
  <button class="row-body" onclick={onselect}>
    <span class="row-title">{task.title}</span>
    <span class="row-meta">
      {#each task.tags as tag (tag)}<span class="meta-tag">#{tag}</span>{/each}
      {#if due}<span class="meta-due" class:overdue>{due}</span>{/if}
    </span>
  </button>
</li>
