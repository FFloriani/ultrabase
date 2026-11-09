<script lang="ts">
  import { api, canSync, errorText, inputFrom } from "../lib/api";
  import { SESSION_TEXT, TOKEN_TEXT, accentStyle, pauseInfo, relative, statusInfo } from "../lib/format";
  import { copy, notify } from "../lib/toast.svelte";
  import type { AccountView, BrowserTarget, ProjectView } from "../lib/types";
  import { copyForAi } from "../lib/ai";
  import ProjectDetails from "./ProjectDetails.svelte";
  import SlotSockets from "./SlotSockets.svelte";

  let {
    account,
    onchanged,
    onedit,
  }: { account: AccountView; onchanged: () => Promise<void>; onedit: () => void } = $props();

  let busy = $state<string | null>(null);
  let shownPassword = $state<string | null>(null);
  let notesDraft = $state("");
  let notesFor = $state("");
  /** Projeto com os detalhes abertos. */
  let openRef = $state<string | null>(null);

  // Ao trocar de conta, o rascunho volta para o texto salvo.
  $effect.pre(() => {
    if (notesFor !== account.id) {
      notesFor = account.id;
      notesDraft = account.notes;
      shownPassword = null;
      openRef = null;
    }
  });

  const sessionText = $derived(SESSION_TEXT[account.sessionStatus]);
  const ready = $derived(canSync(account));

  async function run(label: string, work: () => Promise<unknown>, done?: string) {
    busy = label;
    try {
      await work();
      if (done) notify(done);
    } catch (e) {
      notify(errorText(e), "error");
    } finally {
      busy = null;
      await onchanged();
    }
  }

  function open(target: BrowserTarget = "dashboard", projectRef?: string) {
    api.openBrowser(account.id, target, projectRef).catch((e) => notify(errorText(e), "error"));
  }

  const sync = () => run("sync", () => api.sync(account.id), "Dados e projetos auto-gravados!");

  async function togglePassword() {
    if (shownPassword !== null) {
      shownPassword = null;
      return;
    }
    try {
      shownPassword = (await api.reveal(account.id, "password")) ?? "";
    } catch (e) {
      notify(errorText(e), "error");
    }
  }

  async function copyPassword() {
    try {
      const pw = await api.reveal(account.id, "password");
      if (pw) await copy(pw, "Senha");
    } catch (e) {
      notify(errorText(e), "error");
    }
  }

  async function saveNotes() {
    if (notesDraft === account.notes) return;
    await run("notes", () => api.update(account.id, { ...inputFrom(account), notes: notesDraft }), "Notas salvas");
  }

  function toggleProject(p: ProjectView) {
    const action = p.status === "INACTIVE" ? "restore" : "pause";
    run(
      `${action}:${p.ref}`,
      () => api.projectAction(account.id, p.ref, action),
      action === "pause" ? `${p.name} está pausando` : `${p.name} está acordando`,
    );
  }

  async function copyAi(p: ProjectView) {
    busy = `ai:${p.ref}`;
    await copyForAi(account.id, p.ref);
    busy = null;
  }

  const createToken = () => run("token", () => api.createAiToken(account.id), "Token de acesso criado e guardado no cofre");

  const canToggle = (p: ProjectView) => ["INACTIVE", "ACTIVE_HEALTHY", "ACTIVE_UNHEALTHY"].includes(p.status);
</script>

<section class="detail" style={accentStyle(account.color)}>
  <header>
    <div class="title">
      <span class="chip"></span>
      <div>
        <h1>{account.alias}</h1>
        <p class="meta">
          <span class="tone-{sessionText[1]}">{sessionText[0]}</span>
          <span>{account.email || "o e-mail aparece quando você entrar"}</span>
          {#if account.lastSyncAt}<span>sincronizada {relative(account.lastSyncAt)}</span>{/if}
        </p>
      </div>
    </div>
    <div class="actions">
      <button class="btn primary" onclick={() => open()}>
        {account.browserOpen ? "Mostrar navegador" : "Abrir navegador"}
      </button>
      <button class="btn" onclick={sync} disabled={busy !== null || !ready} title={ready ? "" : "Entre no Supabase pelo navegador da conta primeiro"}>
        {busy === "sync" ? "Gravando…" : "Auto-gravar dados"}
      </button>
      <button class="btn ghost" onclick={onedit}>Editar</button>
    </div>
  </header>

  {#if !ready && account.sessionStatus === "none"}
    <div class="banner warn">
      <div>
        <p class="strong">Entre no Supabase pelo navegador desta conta.</p>
        <p>Use uma conta que você já tem ou crie uma nova ali. O Ultrabase percebe o login sozinho e já lê os projetos.</p>
      </div>
      <button class="btn primary" onclick={() => open("signin")}>
        {account.browserOpen ? "Mostrar navegador" : "Abrir navegador"}
      </button>
    </div>
  {:else if !ready && account.sessionStatus === "expired"}
    <div class="banner danger">
      <div>
        <p class="strong">O login desta conta acabou.</p>
        <p>Abra o navegador dela e entre de novo no Supabase.</p>
      </div>
      <button class="btn primary" onclick={() => open("signin")}>Abrir navegador</button>
    </div>
  {:else if account.lastSyncError}
    <div class="banner warn"><p>Última leitura falhou: {account.lastSyncError}</p></div>
  {/if}

  <section class="block">
    <div class="block-head">
      <h2>Vagas grátis</h2>
      {#if account.lastSyncAt}
        <p class="muted">{account.slotsUsed} de {account.slotLimit} em uso. Projeto pausado libera a vaga.</p>
      {/if}
    </div>
    <SlotSockets {account} large onfree={() => open("dashboard")} />
  </section>

  <section class="block">
    <div class="block-head">
      <h2>Projetos</h2>
      {#if account.organizations.length > 0}
        <p class="muted">
          {account.organizations.length === 1 ? "Organização" : "Organizações"}:
          {account.organizations.map((o) => `${o.name} (${o.plan ?? "plano ?"})`).join(", ")}
        </p>
      {/if}
    </div>

    {#if account.projects.length === 0}
      <p class="empty">
        {account.lastSyncAt
          ? "Nenhum projeto nesta conta ainda."
          : "Os projetos aparecem aqui depois que você entrar no Supabase pelo navegador da conta."}
      </p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Projeto</th>
            <th>Situação</th>
            <th>Pausa</th>
            <th class="end">Ações</th>
          </tr>
        </thead>
        <tbody>
          {#each account.projects as p (p.ref)}
            {@const st = statusInfo(p.status)}
            {@const pz = pauseInfo(p.pause)}
            <tr>
              <td>
                <div class="pname">{p.name}</div>
                <div class="pref mono">{p.ref} <span class="region">{p.region}</span></div>
              </td>
              <td><span class="dot tone-{st[1]}">●</span> {st[0]}</td>
              <td class="tone-{pz[1]}" title={p.lastActivityAt ? `Último uso visto: ${relative(p.lastActivityAt)}` : ""}>
                {pz[0]}
              </td>
              <td class="end row-actions">
                <button class="btn small primary" onclick={() => copyAi(p)} disabled={busy === `ai:${p.ref}`} title="Copia .env completo, conexões do banco e MCP para colar na IA da IDE">
                  {busy === `ai:${p.ref}` ? "Copiando…" : "Copiar para IA"}
                </button>
                <button class="btn small" onclick={() => (openRef = openRef === p.ref ? null : p.ref)} aria-expanded={openRef === p.ref}>
                  {openRef === p.ref ? "Fechar" : "Detalhes"}
                </button>
                <button class="btn small ghost" onclick={() => open("project", p.ref)}>Abrir</button>
                {#if canToggle(p)}
                  <button class="btn small ghost" onclick={() => toggleProject(p)} disabled={busy !== null}>
                    {#if busy === `pause:${p.ref}`}Pausando…{:else if busy === `restore:${p.ref}`}Acordando…{:else}{p.status ===
                      "INACTIVE"
                        ? "Acordar"
                        : "Pausar"}{/if}
                  </button>
                {/if}
              </td>
            </tr>
            {#if openRef === p.ref}
              <tr class="expanded">
                <td colspan="4"><ProjectDetails {account} project={p} {onchanged} /></td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <div class="two">
    <section class="block">
      <div class="block-head"><h2>Acesso</h2></div>
      <dl>
        <dt>Login</dt>
        <dd>
          <span class="value tone-{sessionText[1]}">{sessionText[0]}</span>
          <button class="btn small ghost" onclick={() => open(account.sessionStatus === "connected" ? "dashboard" : "signin")}>
            Abrir navegador
          </button>
        </dd>

        <dt>E-mail</dt>
        <dd>
          <span class="value">{account.email || "aparece quando você entrar"}</span>
          {#if account.email}
            <button class="btn small ghost" onclick={() => copy(account.email, "E-mail")}>Copiar</button>
          {/if}
        </dd>

        <dt>Senha</dt>
        <dd>
          {#if account.hasPassword}
            <span class="value mono">{shownPassword ?? "••••••••••"}</span>
            <button class="btn small ghost" onclick={togglePassword}>{shownPassword !== null ? "Esconder" : "Mostrar"}</button>
            <button class="btn small ghost" onclick={copyPassword}>Copiar</button>
          {:else}
            <span class="value muted">não guardada</span>
            <button class="btn small ghost" onclick={onedit}>Guardar</button>
          {/if}
        </dd>

        <dt>Token</dt>
        <dd>
          {#if account.hasToken}
            {@const tk = TOKEN_TEXT[account.tokenStatus]}
            <span class="value tone-{tk[1]}">{tk[0]}</span>
          {:else}
            <span class="value muted">não criado</span>
            <button
              class="btn small ghost"
              onclick={createToken}
              disabled={busy !== null || account.sessionStatus !== "connected"}
              title="Cria um token de acesso na conta, para o MCP do Supabase na IDE"
            >
              {busy === "token" ? "Criando…" : "Criar token para IA"}
            </button>
          {/if}
        </dd>
      </dl>
    </section>

    <section class="block">
      <div class="block-head"><h2>Notas</h2></div>
      <textarea
        class="input notes"
        bind:value={notesDraft}
        onblur={saveNotes}
        placeholder="Para que serve esta conta, quem usa, o que tem nos projetos…"
      ></textarea>
    </section>
  </div>
</section>

<style>
  .detail {
    max-width: 1040px;
    padding: 26px 32px 48px;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
  }

  .title {
    display: flex;
    gap: 14px;
    min-width: 0;
  }

  .chip {
    flex: none;
    width: 12px;
    height: 40px;
    margin-top: 4px;
    border-radius: 4px;
    background: var(--acc);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.14);
  }

  h1 {
    font-size: 28px;
    line-height: 1.15;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 13px;
  }

  .actions {
    display: flex;
    flex: none;
    gap: 8px;
  }

  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 20px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--warn);
    border-radius: var(--radius-s);
    background: var(--panel);
  }

  .banner.danger {
    border-left-color: var(--danger);
  }

  .banner p {
    margin: 0;
    color: var(--muted);
  }

  .banner p.strong {
    color: var(--text);
    font-weight: 600;
  }

  .block {
    margin-top: 30px;
  }

  .block-head {
    display: flex;
    align-items: baseline;
    gap: 14px;
    margin-bottom: 12px;
  }

  h2 {
    font-size: 16px;
  }

  .muted {
    margin: 0;
    color: var(--faint);
    font-size: 12.5px;
  }

  .empty {
    margin: 0;
    padding: 18px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-m);
    color: var(--faint);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    padding: 0 10px 8px;
    border-bottom: 1px solid var(--line);
    color: var(--faint);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  td {
    padding: 10px;
    border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
    vertical-align: middle;
  }

  .end {
    text-align: right;
  }

  .pname {
    font-weight: 600;
  }

  .pref {
    color: var(--faint);
  }

  .region {
    margin-left: 6px;
    font-family: var(--font);
  }

  .dot {
    font-size: 9px;
    vertical-align: 2px;
  }

  .row-actions {
    white-space: nowrap;
  }

  .row-actions .btn {
    margin-left: 4px;
  }

  tr.expanded td {
    padding-top: 0;
    background: color-mix(in srgb, var(--panel) 60%, transparent);
  }

  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
  }

  dl {
    display: grid;
    grid-template-columns: 70px 1fr;
    gap: 10px 12px;
    margin: 0;
  }

  dt {
    padding-top: 3px;
    color: var(--faint);
    font-size: 12.5px;
  }

  dd {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    margin: 0;
  }

  .value {
    overflow: hidden;
    margin-right: 6px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .notes {
    min-height: 150px;
  }

  @media (max-width: 1100px) {
    .two {
      grid-template-columns: 1fr;
    }

    header {
      flex-direction: column;
    }
  }
</style>
