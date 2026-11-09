<script lang="ts">
  let { onstart, starting }: { onstart: () => void; starting: boolean } = $props();
</script>

<section>
  <h1>Comece pela primeira conta</h1>
  <p>
    Cada conta do Supabase mora num navegador próprio, com login separado das outras. O login fica
    guardado nesse navegador mesmo depois de fechar o app.
  </p>
  <ol>
    <li>Clique em Nova conta: abre um navegador só dela, na página de login do Supabase.</li>
    <li>Entre na sua conta do Supabase nessa janela, ou crie uma conta nova ali.</li>
    <li>Pronto: o Ultrabase percebe o login sozinho e mostra e-mail, projetos e vagas grátis.</li>
  </ol>
  <button class="btn primary" onclick={onstart} disabled={starting}>
    {starting ? "Abrindo navegador…" : "Nova conta"}
  </button>
</section>

<style>
  section {
    max-width: 560px;
    margin: 12vh auto 0;
    padding: 0 24px;
  }

  h1 {
    font-size: 26px;
  }

  p {
    color: var(--muted);
  }

  ol {
    margin: 22px 0 26px;
    padding-left: 20px;
    color: var(--muted);
  }

  li {
    padding: 5px 0 5px 4px;
  }

  li::marker {
    color: var(--text);
    font-weight: 600;
  }
</style>
