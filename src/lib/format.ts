import type { AccountView, LoginMethod, PauseState, SessionStatus, TokenStatus } from "./types";

export type Tone = "ok" | "warn" | "danger" | "muted";

const STATUS: Record<string, [string, Tone]> = {
  ACTIVE_HEALTHY: ["ativo", "ok"],
  ACTIVE_UNHEALTHY: ["ativo com problema", "warn"],
  INACTIVE: ["pausado", "muted"],
  COMING_UP: ["ligando", "warn"],
  RESTORING: ["acordando", "warn"],
  PAUSING: ["pausando", "warn"],
  RESTARTING: ["reiniciando", "warn"],
  UPGRADING: ["atualizando", "warn"],
  RESIZING: ["mudando de tamanho", "warn"],
  PROMOTING: ["promovendo", "warn"],
  GOING_DOWN: ["desligando", "warn"],
  INIT_FAILED: ["falhou ao criar", "danger"],
  RESTORE_FAILED: ["falhou ao acordar", "danger"],
  PAUSE_FAILED: ["falhou ao pausar", "danger"],
  REMOVED: ["removido", "muted"],
};

export function statusInfo(status: string): [string, Tone] {
  return STATUS[status] ?? [status.toLowerCase(), "muted"];
}

export function pauseInfo(pause: PauseState): [string, Tone] {
  switch (pause.kind) {
    case "paused":
      return ["pausado", "muted"];
    case "paid":
      return ["plano pago, não pausa", "muted"];
    case "unknown":
      return ["sem estimativa", "muted"];
    case "days":
      if (pause.days <= 0) return ["pausa a qualquer momento", "danger"];
      if (pause.days === 1) return ["pausa em 1 dia", "danger"];
      if (pause.days <= 2) return [`pausa em ${pause.days} dias`, "warn"];
      return [`pausa em ${pause.days} dias`, "ok"];
  }
}

export const SESSION_TEXT: Record<SessionStatus, [string, Tone]> = {
  none: ["esperando você entrar", "warn"],
  connected: ["conectada", "ok"],
  expired: ["desconectada", "danger"],
};

export const TOKEN_TEXT: Record<TokenStatus, [string, Tone]> = {
  missing: ["sem token", "muted"],
  unchecked: ["token ainda não testado", "warn"],
  ok: ["token funcionando", "ok"],
  invalid: ["token recusado", "danger"],
};

/** Cor do texto em cima da cor da conta: escuro em cor clara, claro em cor escura. */
export function onAccent(hex: string): string {
  const n = parseInt(hex.replace("#", ""), 16);
  if (Number.isNaN(n)) return "#10141b";
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  const lum = 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
  return lum > 0.18 ? "#10141b" : "#f4f6fa";
}

export const accentStyle = (hex: string) => `--acc: ${hex}; --on-acc: ${onAccent(hex)}`;

export const LOGIN_TEXT: Record<LoginMethod, string> = {
  email: "entra com e-mail e senha",
  github: "entra pelo GitHub",
};

export function relative(iso: string | null): string {
  if (!iso) return "nunca";
  const diff = Date.now() - new Date(iso).getTime();
  const min = Math.round(diff / 60000);
  if (min < 1) return "agora";
  if (min < 60) return `há ${min} min`;
  const h = Math.round(min / 60);
  if (h < 24) return `há ${h} h`;
  const d = Math.round(h / 24);
  return d === 1 ? "há 1 dia" : `há ${d} dias`;
}

/** O aviso mais importante da conta, para a lista lateral. */
export function accountAlert(a: AccountView): [string, Tone] | null {
  if (a.sessionStatus === "expired" && a.tokenStatus !== "ok") return ["desconectada", "danger"];
  if (a.sessionStatus === "none" && a.tokenStatus !== "ok") return ["esperando você entrar", "warn"];
  const soonest = a.projects
    .map((p) => (p.pause.kind === "days" ? p.pause.days : Infinity))
    .reduce((min, d) => Math.min(min, d), Infinity);
  if (soonest <= 1) return ["projeto pausa logo", "danger"];
  if (soonest <= 2) return ["projeto pausa em breve", "warn"];
  if (a.lastSyncError) return ["erro na última leitura", "warn"];
  return null;
}

export const ACCOUNT_COLORS = [
  "#62c9a5",
  "#e9b44c",
  "#ef7b6b",
  "#8fb4ff",
  "#c792ea",
  "#f28fb5",
  "#7fd1e8",
  "#b5d36b",
];
