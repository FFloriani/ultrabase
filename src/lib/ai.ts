import { api, errorText } from "./api";
import { copy, notify } from "./toast.svelte";

/** Busca tudo do projeto e copia o texto pronto para a IA da IDE. */
export async function copyForAi(accountId: string, projectRef: string): Promise<boolean> {
  try {
    const s = await api.projectSecrets(accountId, projectRef);
    await navigator.clipboard.writeText(s.bundle);
    const faltam = [!s.dbPassword && "a senha do banco", !s.accessToken && "o token do MCP"].filter(Boolean);
    if (faltam.length) {
      notify(`Copiado para a IA. Ainda falta ${faltam.join(" e ")}: veja em Detalhes`, "error");
    } else {
      notify("Tudo do projeto copiado para a IA");
    }
    return true;
  } catch (e) {
    notify(errorText(e), "error");
    return false;
  }
}

export { copy };
