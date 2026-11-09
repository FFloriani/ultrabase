<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, errorText } from "./lib/api";
  import { ACCOUNT_COLORS } from "./lib/format";
  import { notify } from "./lib/toast.svelte";
  import type { AccountView, Overview } from "./lib/types";
  import AccountDetail from "./components/AccountDetail.svelte";
  import AccountForm from "./components/AccountForm.svelte";
  import EmptyState from "./components/EmptyState.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Toasts from "./components/Toasts.svelte";

  let overview = $state<Overview | null>(null);
  let loadError = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  let editing = $state<AccountView | null>(null);
  let syncingAll = $state(false);
  let starting = $state(false);

  const selected = $derived(overview?.accounts.find((a) => a.id === selectedId) ?? null);

  async function load() {
    try {
      overview = await api.list();
      loadError = null;
      if (!overview.accounts.some((a) => a.id === selectedId)) {
        selectedId = overview.accounts[0]?.id ?? null;
      }
    } catch (e) {
      loadError = errorText(e);
    }
  }

  /** Nova conta = navegador novo, já na página de login do Supabase. */
  async function startAccount() {
    starting = true;
    try {
      const used = overview?.accounts.map((a) => a.color) ?? [];
      const color = ACCOUNT_COLORS.find((c) => !used.includes(c)) ?? ACCOUNT_COLORS[used.length % ACCOUNT_COLORS.length];
      const view = await api.start(color);
      selectedId = view.id;
      notify(`Navegador da ${view.alias} aberto: entre ou crie a conta no Supabase nele`);
      await load();
    } catch (e) {
      notify(errorText(e), "error");
    } finally {
      starting = false;
    }
  }

  async function syncAll() {
    syncingAll = true;
    try {
      overview = await api.syncAll();
      const failed = overview.accounts.filter((a) => a.lastSyncError).length;
      notify(failed ? `${failed} conta(s) com erro na leitura` : "Todas as contas sincronizadas", failed ? "error" : "ok");
    } catch (e) {
      notify(errorText(e), "error");
    } finally {
      syncingAll = false;
    }
  }

  onMount(() => {
    load();
    const stops = [
      listen<{ accountId: string; open: boolean }>("browser-changed", (e) => {
        const acc = overview?.accounts.find((a) => a.id === e.payload.accountId);
        if (acc) acc.browserOpen = e.payload.open;
      }),
      listen("session-changed", () => load()),
      // O vigia do navegador terminou a primeira leitura depois do login.
      listen<{ accountId: string; error: string | null }>("account-synced", async (e) => {
        await load();
        const acc = overview?.accounts.find((a) => a.id === e.payload.accountId);
        if (!acc) return;
        if (e.payload.error) notify(`${acc.alias}: ${e.payload.error}`, "error");
        else notify(`${acc.alias} conectada${acc.email ? ` como ${acc.email}` : ""}`);
      }),
    ];
    // Voltar para o app (vindo do navegador de uma conta, por exemplo) relê os dados locais.
    window.addEventListener("focus", load);
    return () => {
      stops.forEach((p) => p.then((stop) => stop()));
      window.removeEventListener("focus", load);
    };
  });
</script>

<div class="shell">
  {#if overview}
    <Sidebar
      {overview}
      {selectedId}
      {syncingAll}
      {starting}
      onselect={(id) => (selectedId = id)}
      onstart={startAccount}
      onsyncall={syncAll}
    />
    <main>
      {#if selected}
        <AccountDetail account={selected} onchanged={load} onedit={() => (editing = selected)} />
      {:else}
        <EmptyState onstart={startAccount} {starting} />
      {/if}
    </main>
  {:else if loadError}
    <p class="fatal">Não deu para abrir os dados do app: {loadError}</p>
  {/if}
</div>

{#if editing}
  <AccountForm
    account={editing}
    onclose={() => (editing = null)}
    onsaved={async (view) => {
      editing = null;
      selectedId = view.id;
      await load();
    }}
    ondeleted={async () => {
      editing = null;
      selectedId = null;
      await load();
    }}
  />
{/if}

<Toasts />

<style>
  .shell {
    display: grid;
    grid-template-columns: 290px 1fr;
    height: 100%;
  }

  main {
    overflow-y: auto;
    min-width: 0;
  }

  .fatal {
    grid-column: 1 / -1;
    padding: 40px;
    color: var(--danger);
  }
</style>
