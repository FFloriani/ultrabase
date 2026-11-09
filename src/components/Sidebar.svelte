<script lang="ts">
  import { canSync } from "../lib/api";
  import { accentStyle, accountAlert } from "../lib/format";
  import type { Overview } from "../lib/types";
  import SlotSockets from "./SlotSockets.svelte";

  let {
    overview,
    selectedId,
    syncingAll,
    starting,
    onselect,
    onstart,
    onsyncall,
  }: {
    overview: Overview;
    selectedId: string | null;
    syncingAll: boolean;
    starting: boolean;
    onselect: (id: string) => void;
    onstart: () => void;
    onsyncall: () => void;
  } = $props();

  const suggested = $derived(overview.accounts.find((a) => a.id === overview.suggestedAccountId));
  const readyCount = $derived(overview.accounts.filter((a) => canSync(a) && a.lastSyncAt).length);
</script>

<aside>
  <header>
    <div class="brand">
      <svg viewBox="0 0 32 32" width="22" height="22" aria-hidden="true">
        <rect x="3" y="5" width="26" height="6" rx="2.5" fill="#62c9a5" />
        <rect x="6" y="13" width="23" height="6" rx="2.5" fill="#e9b44c" />
        <rect x="9" y="21" width="20" height="6" rx="2.5" fill="#ef7b6b" />
      </svg>
      <span>Ultrabase</span>
    </div>

    {#if overview.accounts.length > 0}
      <p class="summary">
        <strong>{overview.totalFreeSlots}</strong>
        {overview.totalFreeSlots === 1 ? "vaga grátis livre" : "vagas grátis livres"}
        {#if readyCount === 0}
          <span class="hint">nenhuma conta conectada ainda; entre no Supabase pelo navegador da conta</span>
        {:else if readyCount < overview.accounts.length}
          <span class="hint">
            contando só {readyCount === 1 ? "a conta conectada" : `as ${readyCount} contas conectadas`}
          </span>
        {/if}
      </p>
      {#if suggested}
        <button class="suggest" onclick={() => onselect(suggested.id)} style={accentStyle(suggested.color)}>
          Próximo projeto vai em <b>{suggested.alias}</b>
        </button>
      {/if}
    {/if}
  </header>

  <nav aria-label="Contas">
    {#each overview.accounts as a (a.id)}
      {@const alert = accountAlert(a)}
      <button
        class="account"
        class:selected={a.id === selectedId}
        style={accentStyle(a.color)}
        onclick={() => onselect(a.id)}
        aria-current={a.id === selectedId ? "true" : undefined}
      >
        <span class="bar"></span>
        <span class="text">
          <span class="alias">{a.alias}</span>
          <span class="sub">
            {#if alert}
              <span class="tone-{alert[1]}">{alert[0]}</span>
            {:else}
              {a.email || "conectada"}
            {/if}
          </span>
        </span>
        <span class="right">
          <SlotSockets account={a} />
          {#if a.browserOpen}<span class="open-dot" title="Navegador aberto"></span>{/if}
        </span>
      </button>
    {/each}
  </nav>

  <footer>
    <button class="btn" onclick={onstart} disabled={starting} title="Abre um navegador novo, só para essa conta">
      {starting ? "Abrindo…" : "Nova conta"}
    </button>
    {#if overview.accounts.length > 0}
      <button class="btn ghost" onclick={onsyncall} disabled={syncingAll}>
        {syncingAll ? "Sincronizando…" : "Sincronizar todas"}
      </button>
    {/if}
  </footer>
</aside>

<style>
  aside {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100%;
    border-right: 1px solid var(--line);
    background: var(--panel);
    min-height: 0;
  }

  header {
    padding: 18px 16px 14px;
    border-bottom: 1px solid var(--line);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 700;
  }

  .summary {
    margin: 18px 0 0;
    color: var(--muted);
  }

  .summary strong {
    display: block;
    color: var(--text);
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 650;
    line-height: 1.1;
  }

  .hint {
    display: block;
    margin-top: 2px;
    color: var(--faint);
    font-size: 12px;
  }

  .suggest {
    width: 100%;
    margin-top: 12px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--acc);
    border-radius: var(--radius-s);
    background: var(--bg);
    color: var(--muted);
    text-align: left;
    font-size: 12.5px;
  }

  .suggest b {
    color: var(--text);
  }

  nav {
    overflow-y: auto;
    padding: 8px;
    min-height: 0;
  }

  .account {
    display: grid;
    grid-template-columns: 4px 1fr auto;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px 9px 6px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    text-align: left;
  }

  .account:hover {
    background: var(--panel-2);
  }

  .account.selected {
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .bar {
    align-self: stretch;
    border-radius: 2px;
    background: var(--acc);
    opacity: 0.55;
  }

  .selected .bar {
    opacity: 1;
  }

  .text {
    display: grid;
    min-width: 0;
  }

  .alias {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    overflow: hidden;
    color: var(--faint);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .open-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--acc);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--acc) 25%, transparent);
  }

  footer {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
</style>
