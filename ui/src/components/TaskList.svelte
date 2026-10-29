<script lang="ts">
  import { app } from '../lib/state.svelte';
  import TaskRow from './TaskRow.svelte';

  const emptyText = $derived(
    app.searching
      ? '검색 결과가 없어요'
      : app.tag
        ? `#${app.tag} 태그의 할 일이 없어요`
        : { today: '오늘 할 일이 없어요', upcoming: '예정된 할 일이 없어요', overdue: '지난 일이 없어요', all: '할 일이 없어요', completed: '완료한 일이 아직 없어요' }[app.view],
  );
</script>

<section class="list" aria-label="할 일 목록">
  {#if app.mainTasks.length === 0 && app.doneTasks.length === 0}
    <p class="empty">{emptyText}</p>
  {/if}

  <ul class="rows">
    {#each app.mainTasks as task (task.id)}
      <TaskRow
        {task}
        selected={app.selectedId === task.id}
        onselect={() => (app.selectedId = task.id)}
        ontoggle={() => app.toggle(task.id)}
      />
    {/each}
  </ul>

  {#if app.view !== 'completed' || app.searching}
    {#if app.doneTasks.length > 0}
      <button class="done-toggle" onclick={() => (app.doneOpen = !app.doneOpen)}>
        {app.doneOpen ? '▾' : '▸'} 완료 {app.doneTasks.length}개
      </button>
      {#if app.doneOpen}
        <ul class="rows">
          {#each app.doneTasks as task (task.id)}
            <TaskRow
              {task}
              selected={app.selectedId === task.id}
              onselect={() => (app.selectedId = task.id)}
              ontoggle={() => app.toggle(task.id)}
            />
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</section>
