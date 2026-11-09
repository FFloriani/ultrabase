<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../lib/api";
  import { copyForAi } from "../lib/ai";
  import { copy, notify } from "../lib/toast.svelte";
  import type { AccountView, ProjectSecrets, ProjectView } from "../lib/types";

  let {
    account,
    project,
    onchanged,
  }: { account: AccountView; project: ProjectView; onchanged: () => Promise<void> } = $props();

  let secrets = $state<ProjectSecrets | null>(null);
  let loadError = $state<string | null>(null);
  let shown = $state<Record<string, boolean>>({});
  let passwordDraft = $state("");
  let busy = $state<string | null>(null);
  let confirmReset = $state(false);

  async function load() {
    try {
      secrets = await api.projectSecrets(account.id, project.ref);
      loadError = null;
    } catch (e) {
      loadError = errorText(e);
    }
  }

  onMount(load);

  const mask = (v: string) => "•".repeat(Math.min(28, Math.max(10, Math.round(v.length / 8))));
  /** Endereço com a senha escondida na tela; a cópia leva a senha de verdade. */
  const hidePassword = (url: string) => url.replace(/(:\/\/[^:]+:)([^@]+)(@)/, (_m, a, pw, b) => a + (pw === "[YOUR-PASSWORD]" ? pw : "••••••••") + b);

  async function savePassword(e: SubmitEvent) {
    e.preventDefault();
    if (!passwordDraft.trim()) return;
    busy = "save";
    try {
      await api.setDbPassword(account.id, project.ref, passwordDraft);
      passwordDraft = "";
      notify("Senha do banco guardada no cofre do Windows");
      await load();
    } catch (err) {
      notify(errorText(err), "error");
    } finally {
      busy = null;
    }
  }

  async function resetPassword() {
    if (!confirmReset) {
      confirmReset = true;
      setTimeout(() => (confirmReset = false), 6000);
      return;
    }
    confirmReset = false;
    busy = "reset";
    try {
      await api.resetDbPassword(account.id, project.ref);
      notify("Senha nova do banco criada no Supabase e guardada no cofre");
      await load();
    } catch (err) {
      notify(errorText(err), "error");
    } finally {
      busy = null;
      await onchanged();
    }
  }

  async function createToken() {
    busy = "token";
    try {
      await api.createAiToken(account.id);
      notify("Token de acesso criado na conta e guardado no cofre");
      await load();
    } catch (err) {
      notify(errorText(err), "error");
    } finally {
      busy = null;
      await onchanged();
    }
  }

  async function copyAll() {
    busy = "ai";
    await copyForAi(account.id, project.ref);
    busy = null;
  }

  type Row = { key: string; label: string; value: string | null; secret?: boolean; hint?: string; display?: string };

  const appRows = $derived<Row[]>(
    secrets
      ? [
          { key: "url", label: "Endereço da API", value: secrets.apiUrl },
          { key: "anon", label: "Chave anon", value: secrets.anonKey, hint: "pública: pode ir para o navegador" },
          { key: "pub", label: "Chave publishable", value: secrets.publishableKey, hint: "pública, formato novo" },
        ]
      : [],
  );

  const secretRows = $derived<Row[]>(
    secrets
      ? [
          { key: "service", label: "service_role", value: secrets.serviceRoleKey, secret: true },
          { key: "secret", label: "Chave secret", value: secrets.secretKey, secret: true, hint: "formato novo da service_role" },
          { key: "jwt", label: "JWT secret", value: secrets.jwtSecret, secret: true },
        ]
      : [],
  );

  const connRows = $derived<Row[]>(
    secrets
      ? [
          { key: "tx", label: "Pooler, transação", value: secrets.poolerTransactionUrl, hint: "porta 6543: para o app (DATABASE_URL)" },
          { key: "session", label: "Pooler, sessão", value: secrets.poolerSessionUrl, hint: "porta 5432: para migrações (DIRECT_URL)" },
          {
            key: "direct",
            label: "Direta",
            value: secrets.directUrl,
            hint: secrets.ipv6Only ? "só funciona em rede com IPv6" : "conexão direta ao banco",
          },
        ].map((r) => ({ ...r, display: r.value ? hidePassword(r.value) : undefined }))
      : [],
  );
</script>

<div class="details">
  {#if loadError}
    <p class="error">Não deu para ler os dados do projeto: {loadError}</p>
  {:else if !secrets}
    <p class="loading">Lendo chaves e conexões no Supabase…</p>
  {:else}
    <div class="ai">
      <div>
        <p class="strong">Tudo deste projeto para a IA da sua IDE</p>
        <p class="muted">
          Copia um texto com o .env completo (inclusive as chaves secretas), as conexões do banco e a configuração do
          MCP do Supabase. Cole no chat do Cursor, Claude Code ou outra IDE.
        </p>
      </div>
      <button class="btn primary" onclick={copyAll} disabled={busy === "ai"}>
        {busy === "ai" ? "Copiando…" : "Copiar para IA"}
      </button>
    </div>

    <div class="groups">
      <section>
        <h3>Para o app</h3>
        {#each appRows as r (r.key)}
          {@render row(r)}
        {/each}
      </section>

      <section>
        <h3>Secretas <span class="warn">só no servidor, nunca no navegador nem no git</span></h3>
        {#each secretRows as r (r.key)}
          {@render row(r)}
        {/each}
      </section>

      <section>
        <h3>Banco</h3>
        <div class="row">
          <span class="label">Senha do banco</span>
          {#if secrets.dbPassword}
            <span class="value mono">{shown.db ? secrets.dbPassword : mask(secrets.dbPassword)}</span>
            <span class="actions">
              <button class="btn small ghost" onclick={() => (shown.db = !shown.db)}>{shown.db ? "Esconder" : "Mostrar"}</button>
              <button class="btn small ghost" onclick={() => copy(secrets!.dbPassword!, "Senha do banco")}>Copiar</button>
              <button class="btn small ghost danger" onclick={resetPassword} disabled={busy === "reset"}>
                {busy === "reset" ? "Trocando…" : confirmReset ? "Confirmar: a senha antiga para de funcionar" : "Gerar outra"}
              </button>
            </span>
          {:else}
            <form class="password-form" onsubmit={savePassword}>
              <input
                class="input mono"
                type="password"
                placeholder="Cole a senha que você criou no projeto"
                bind:value={passwordDraft}
                autocomplete="off"
              />
              <button class="btn small" type="submit" disabled={!passwordDraft.trim() || busy !== null}>Guardar</button>
              <button type="button" class="btn small ghost" onclick={resetPassword} disabled={busy === "reset"}>
                {busy === "reset" ? "Gerando…" : confirmReset ? "Confirmar: troca a senha no Supabase" : "Não sei: gerar nova"}
              </button>
            </form>
          {/if}
        </div>
        <p class="note">O Supabase não guarda a senha do banco; por isso ela fica no cofre do Windows, só neste PC.</p>
        {#each connRows as r (r.key)}
          {@render row(r)}
        {/each}
      </section>

      <section>
        <h3>MCP do Supabase na IDE</h3>
        <div class="row">
          <span class="label">Token de acesso</span>
          {#if secrets.accessToken}
            <span class="value mono">{shown.token ? secrets.accessToken : mask(secrets.accessToken)}</span>
            <span class="actions">
              <button class="btn small ghost" onclick={() => (shown.token = !shown.token)}>{shown.token ? "Esconder" : "Mostrar"}</button>
              <button class="btn small ghost" onclick={() => copy(secrets!.accessToken!, "Token de acesso")}>Copiar</button>
            </span>
          {:else}
            <span class="value muted">ainda não criado</span>
            <span class="actions">
              <button class="btn small" onclick={createToken} disabled={busy === "token"}>
                {busy === "token" ? "Criando…" : "Criar token para IA"}
              </button>
            </span>
          {/if}
        </div>
        <p class="note">
          Um token por conta, igual ao de Account, Access Tokens no Supabase. Com ele a IA da IDE lê e mexe no projeto
          pelo MCP.
        </p>
      </section>
    </div>
  {/if}
</div>

{#snippet row(r: Row)}
  <div class="row">
    <span class="label">{r.label}</span>
    {#if r.value}
      <span class="value mono" title={r.hint ?? ""}>
        {r.secret && !shown[r.key] ? mask(r.value) : (r.display ?? r.value)}
      </span>
      <span class="actions">
        {#if r.secret}
          <button class="btn small ghost" onclick={() => (shown[r.key] = !shown[r.key])}>{shown[r.key] ? "Esconder" : "Mostrar"}</button>
        {/if}
        <button class="btn small ghost" onclick={() => copy(r.value!, r.label)}>Copiar</button>
      </span>
    {:else}
      <span class="value muted">não existe neste projeto</span>
    {/if}
    {#if r.hint}<span class="hint">{r.hint}</span>{/if}
  </div>
{/snippet}

<style>
  .details {
    padding: 4px 0 18px;
  }

  .loading,
  .error {
    margin: 0;
    padding: 12px 0;
    color: var(--faint);
  }

  .error {
    color: var(--danger);
  }

  .ai {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--acc);
    border-radius: var(--radius-s);
    background: var(--bg);
  }

  .ai p {
    margin: 0;
  }

  .strong {
    font-weight: 600;
  }

  .muted {
    color: var(--faint);
    font-size: 12.5px;
  }

  .groups {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 28px;
    margin-top: 14px;
  }

  h3 {
    margin: 10px 0 6px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }

  h3 .warn {
    margin-left: 6px;
    color: var(--warn);
    font-weight: 400;
    font-size: 12px;
  }

  .row {
    display: grid;
    grid-template-columns: 130px minmax(0, 1fr) auto;
    align-items: center;
    gap: 2px 10px;
    padding: 4px 0;
  }

  .label {
    color: var(--faint);
    font-size: 12.5px;
  }

  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .value.muted {
    font-size: 12.5px;
  }

  .actions {
    display: flex;
    gap: 2px;
    white-space: nowrap;
  }

  .hint {
    grid-column: 2 / -1;
    color: var(--faint);
    font-size: 11.5px;
  }

  .note {
    margin: 2px 0 6px;
    color: var(--faint);
    font-size: 11.5px;
  }

  .password-form {
    display: flex;
    flex-wrap: wrap;
    grid-column: 2 / -1;
    gap: 6px;
  }

  /* O campo ocupa a linha toda; os botões descem para a linha de baixo. */
  .password-form .input {
    flex: 1 1 100%;
    height: 30px;
  }

  @media (max-width: 1200px) {
    .groups {
      grid-template-columns: 1fr;
    }
  }
</style>
