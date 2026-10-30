<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '../lib/state.svelte';

  let cancelBtn: HTMLButtonElement | undefined = $state();
  let okBtn: HTMLButtonElement | undefined = $state();

  onMount(() => cancelBtn?.focus()); // 파괴적인 동작이므로 기본 포커스는 취소

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    e.preventDefault();
    const target = document.activeElement === cancelBtn ? okBtn : cancelBtn;
    target?.focus();
  }
</script>

{#if app.confirmState}
  <button class="backdrop confirm-backdrop" aria-label="취소" tabindex="-1" onclick={() => app.answerConfirm(false)}></button>
  <div
    class="dialog confirm"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-message"
    tabindex="-1"
    {onkeydown}
  >
    <p class="confirm-message" id="confirm-message">{app.confirmState.message}</p>
    <div class="dialog-footer">
      <button bind:this={cancelBtn} onclick={() => app.answerConfirm(false)}>취소</button>
      <button bind:this={okBtn} class="danger-solid" onclick={() => app.answerConfirm(true)}>
        {app.confirmState.confirmLabel}
      </button>
    </div>
  </div>
{/if}
