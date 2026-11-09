<script lang="ts">
  import { accentStyle } from "../lib/format";
  import type { AccountView } from "../lib/types";

  let {
    account,
    large = false,
    onfree,
  }: { account: AccountView; large?: boolean; onfree?: () => void } = $props();

  const used = $derived(account.projects.filter((p) => p.countsTowardLimit));
  const free = $derived(Math.max(account.slotLimit - used.length, 0));
  // Conta que nunca foi lida: não dá para dizer quantas vagas tem.
  const unknown = $derived(account.lastSyncAt === null);
</script>

{#if unknown}
  {#if large}
    <p class="unknown">Sincronize a conta para ver as vagas.</p>
  {:else}
    <span class="unknown-small" title="Vagas ainda não lidas">?</span>
  {/if}
{:else}
<div class="sockets" class:large style={accentStyle(account.color)}>
  {#each used as p (p.ref)}
    <span class="socket used" title="{p.name} ocupa uma vaga grátis">
      {#if large}<span class="name">{p.name}</span>{/if}
    </span>
  {/each}
  {#each { length: free }, i (i)}
    {#if large && onfree}
      <button class="socket free" onclick={onfree} title="Abre o painel desta conta para criar um projeto">
        vaga livre
      </button>
    {:else}
      <span class="socket free" title="Vaga grátis livre">
        {#if large}vaga livre{/if}
      </span>
    {/if}
  {/each}
</div>
{/if}

<style>
  .unknown {
    margin: 0;
    padding: 12px 14px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
    color: var(--faint);
  }

  .unknown-small {
    color: var(--faint);
    font-size: 12px;
  }

  .sockets {
    display: flex;
    gap: 4px;
  }

  .socket {
    width: 14px;
    height: 8px;
    border-radius: 3px;
  }

  .socket.used {
    background: var(--acc);
  }

  .socket.free {
    border: 1.5px dashed var(--faint);
    background: transparent;
  }

  .large {
    gap: 10px;
  }

  .large .socket {
    display: flex;
    align-items: center;
    width: 220px;
    height: 44px;
    padding: 0 14px;
    border-radius: var(--radius-m);
    font-size: 13px;
  }

  .large .socket.used {
    color: var(--on-acc);
    font-weight: 600;
    box-shadow: inset 0 -3px 0 rgb(0 0 0 / 0.18);
  }

  .large .socket.used .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .large .socket.free {
    border: 1.5px dashed var(--line);
    color: var(--faint);
  }

  button.socket.free:hover {
    border-color: var(--acc);
    color: var(--text);
  }
</style>
