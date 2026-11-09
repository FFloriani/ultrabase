<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, errorText } from "../lib/api";
  import { ACCOUNT_COLORS, accentStyle } from "../lib/format";
  import { notify } from "../lib/toast.svelte";
  import type { AccountView, LoginMethod } from "../lib/types";

  let {
    account,
    onclose,
    onsaved,
    ondeleted,
  }: {
    account: AccountView;
    onclose: () => void;
    onsaved: (view: AccountView) => void;
    ondeleted: (id: string) => void;
  } = $props();

  // O formulário copia a conta uma vez, ao abrir; editar não mexe na conta até salvar.
  const start = untrack(() => account);

  let alias = $state(start.alias);
  let color = $state(start.color);
  let email = $state(start.email);
  let loginMethod = $state<LoginMethod>(start.loginMethod);
  let notes = $state(start.notes);
  let password = $state("");
  let token = $state("");
  let removePassword = $state(false);
  let removeToken = $state(false);
  let saving = $state(false);
  let confirmDelete = $state(false);

  let aliasInput: HTMLInputElement | undefined = $state();
  onMount(() => aliasInput?.focus());

  async function save(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    const input = {
      alias,
      color,
      email,
      loginMethod,
      notes,
      password: removePassword ? "" : password.trim() ? password : null,
      token: removeToken ? "" : token.trim() ? token.trim() : null,
    };
    try {
      const view = await api.update(start.id, input);
      notify("Conta salva");
      onsaved(view);
    } catch (err) {
      notify(errorText(err), "error");
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      setTimeout(() => (confirmDelete = false), 5000);
      return;
    }
    try {
      await api.remove(start.id);
      notify(`${start.alias} apagada do Ultrabase`);
      ondeleted(start.id);
    } catch (err) {
      notify(errorText(err), "error");
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <form class="dialog" onsubmit={save} style={accentStyle(color)} aria-label="Editar conta">
    <h2>Editar {start.alias}</h2>

    <label class="field">
      <span>Apelido</span>
      <input class="input" bind:value={alias} bind:this={aliasInput} maxlength="60" required placeholder="Ex.: Pessoal, Cliente Loja" />
    </label>

    <div class="field">
      <span>Cor</span>
      <div class="swatches">
        {#each ACCOUNT_COLORS as c (c)}
          <button
            type="button"
            class="swatch"
            class:on={color === c}
            style="background: {c}"
            onclick={() => (color = c)}
            aria-label="Cor {c}"
            aria-pressed={color === c}
          ></button>
        {/each}
        <input type="color" bind:value={color} aria-label="Outra cor" title="Outra cor" />
      </div>
    </div>

    <label class="field">
      <span>E-mail da conta</span>
      <input class="input" type="email" bind:value={email} placeholder="Preenchido sozinho quando você entra no Supabase" />
    </label>

    <div class="field">
      <span>Como entra no Supabase (só para lembrar)</span>
      <div class="segmented" role="radiogroup">
        <button type="button" role="radio" aria-checked={loginMethod === "email"} class:on={loginMethod === "email"} onclick={() => (loginMethod = "email")}>
          E-mail e senha
        </button>
        <button type="button" role="radio" aria-checked={loginMethod === "github"} class:on={loginMethod === "github"} onclick={() => (loginMethod = "github")}>
          GitHub
        </button>
      </div>
    </div>

    <details class="extra" open={start.hasPassword || start.hasToken}>
      <summary>Senha e token (opcional)</summary>
      <div class="row">
        <label class="field">
          <span>Senha {loginMethod === "github" ? "do GitHub" : "do Supabase"}</span>
          <input
            class="input"
            type="password"
            bind:value={password}
            disabled={removePassword}
            autocomplete="new-password"
            placeholder={start.hasPassword ? "Vazio mantém a senha guardada" : "Fica no cofre do Windows"}
          />
          {#if start.hasPassword}
            <label class="check"><input type="checkbox" bind:checked={removePassword} /> apagar a senha guardada</label>
          {/if}
        </label>

        <label class="field">
          <span>Token de acesso</span>
          <input
            class="input mono"
            type="password"
            bind:value={token}
            disabled={removeToken}
            autocomplete="off"
            spellcheck="false"
            placeholder={start.hasToken ? "Vazio mantém o token guardado" : "sbp_…"}
          />
          {#if start.hasToken}
            <label class="check"><input type="checkbox" bind:checked={removeToken} /> apagar o token guardado</label>
          {:else}
            <small>Só serve para ler a conta quando o login do navegador acabar.</small>
          {/if}
        </label>
      </div>
    </details>

    <label class="field">
      <span>Notas</span>
      <textarea class="input" bind:value={notes}></textarea>
    </label>

    <div class="buttons">
      <button type="button" class="btn ghost danger" onclick={remove}>
        {confirmDelete ? "Clique de novo para apagar" : "Apagar conta"}
      </button>
      <span class="spacer"></span>
      <button type="button" class="btn ghost" onclick={onclose}>Cancelar</button>
      <button type="submit" class="btn primary" disabled={saving || !alias.trim()}>
        {saving ? "Salvando…" : "Salvar"}
      </button>
    </div>
    {#if confirmDelete}
      <p class="warning">
        Apaga a conta, o navegador dela (com o login), a senha e o token daqui do Ultrabase. A conta no Supabase continua existindo.
      </p>
    {/if}
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(8 10 14 / 0.6);
  }

  .dialog {
    display: grid;
    gap: 16px;
    width: min(560px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 24px;
    border: 1px solid var(--line);
    border-top: 3px solid var(--acc);
    border-radius: var(--radius-m);
    background: var(--panel);
    box-shadow: 0 24px 60px rgb(0 0 0 / 0.45);
  }

  h2 {
    font-size: 19px;
  }

  .swatches {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .swatch {
    width: 24px;
    height: 24px;
    border: 2px solid transparent;
    border-radius: 6px;
  }

  .swatch.on {
    border-color: var(--text);
  }

  input[type="color"] {
    width: 30px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: none;
  }

  .segmented {
    display: inline-flex;
    width: fit-content;
    padding: 3px;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    background: var(--bg);
  }

  .segmented button {
    height: 28px;
    padding: 0 14px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
  }

  .segmented button.on {
    background: var(--panel-2);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .extra summary {
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
  }

  .extra[open] summary {
    margin-bottom: 12px;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12.5px;
  }

  small {
    color: var(--faint);
    font-size: 12px;
  }

  .buttons {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .spacer {
    flex: 1;
  }

  .warning {
    margin: 0;
    color: var(--danger);
    font-size: 12.5px;
  }
</style>
