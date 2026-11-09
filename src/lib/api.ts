import { invoke } from "@tauri-apps/api/core";
import type { AccountInput, AccountView, BrowserTarget, Overview, ProjectSecrets } from "./types";

export const api = {
  list: () => invoke<Overview>("list_accounts"),
  /** Cria a conta e já abre o navegador dela no login do Supabase. */
  start: (color: string) => invoke<AccountView>("start_account", { color }),
  update: (id: string, input: AccountInput) => invoke<AccountView>("update_account", { id, input }),
  remove: (id: string) => invoke<void>("delete_account", { id }),
  reveal: (id: string, kind: "password" | "token") => invoke<string | null>("reveal_secret", { id, kind }),
  sync: (id: string) => invoke<AccountView>("sync_account", { id }),
  syncAll: () => invoke<Overview>("sync_all"),
  openBrowser: (id: string, target: BrowserTarget, projectRef?: string) =>
    invoke<void>("open_browser", { id, target, projectRef: projectRef ?? null }),
  projectAction: (id: string, projectRef: string, action: "pause" | "restore") =>
    invoke<AccountView>("project_action", { id, projectRef, action }),
  projectSecrets: (id: string, projectRef: string) => invoke<ProjectSecrets>("project_secrets", { id, projectRef }),
  setDbPassword: (id: string, projectRef: string, password: string) =>
    invoke<void>("set_db_password", { id, projectRef, password }),
  resetDbPassword: (id: string, projectRef: string) => invoke<void>("reset_db_password", { id, projectRef }),
  createAiToken: (id: string) => invoke<AccountView>("create_ai_token", { id }),
};

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** Dados atuais da conta no formato do formulário, sem mexer em senha e token. */
export function inputFrom(a: AccountView): AccountInput {
  return { alias: a.alias, color: a.color, email: a.email, loginMethod: a.loginMethod, notes: a.notes };
}

export const canSync = (a: AccountView) => a.sessionStatus === "connected" || a.tokenStatus === "ok";
