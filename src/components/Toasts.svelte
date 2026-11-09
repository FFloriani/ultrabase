<script lang="ts">
  import { dismiss, toasts } from "../lib/toast.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <button class="toast {t.kind}" onclick={() => dismiss(t.id)} title="Fechar">{t.text}</button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 50;
    display: grid;
    gap: 8px;
    max-width: 420px;
  }

  .toast {
    padding: 10px 14px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--ok);
    border-radius: var(--radius-s);
    background: var(--panel-2);
    text-align: left;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
    animation: rise 160ms ease-out;
  }

  .toast.error {
    border-left-color: var(--danger);
  }

  @keyframes rise {
    from {
      transform: translateY(6px);
      opacity: 0;
    }
  }
</style>
