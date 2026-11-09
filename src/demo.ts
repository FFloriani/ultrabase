// Modo demonstração (só em desenvolvimento, com ?demo no endereço): dados de mentira
// para ver todas as situações da tela no navegador, sem o app Tauri.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { AccountView, Overview, ProjectView } from "./lib/types";

const HOUR = 3_600_000;
const ago = (ms: number) => new Date(Date.now() - ms).toISOString();

function project(p: Partial<ProjectView> & Pick<ProjectView, "ref" | "name" | "status" | "pause">): ProjectView {
  return {
    region: "sa-east-1",
    organizationId: "org",
    createdAt: ago(40 * 24 * HOUR),
    lastActivityAt: null,
    usageChecked: true,
    organizationName: "Org",
    plan: "free",
    countsTowardLimit: p.status !== "INACTIVE",
    apiUrl: `https://${p.ref}.supabase.co`,
    ...p,
  };
}

function account(a: Partial<AccountView> & Pick<AccountView, "id" | "alias" | "color" | "projects">): AccountView {
  const used = a.projects.filter((p) => p.countsTowardLimit).length;
  return {
    email: "",
    loginMethod: "email",
    notes: "",
    username: null,
    sessionStatus: "connected",
    tokenStatus: "missing",
    lastSyncAt: ago(0.2 * HOUR),
    lastSyncError: null,
    createdAt: ago(60 * 24 * HOUR),
    hasPassword: true,
    hasToken: false,
    organizations: [{ id: "org", slug: "org", name: "Org", plan: "free" }],
    slotLimit: 2,
    slotsUsed: used,
    slotsFree: Math.max(2 - used, 0),
    browserOpen: false,
    ...a,
  };
}

const accounts: AccountView[] = [
  account({
    id: "a1",
    alias: "Pessoal",
    color: "#62c9a5",
    email: "floriani.pessoal@exemplo.com",
    loginMethod: "github",
    browserOpen: true,
    notes: "Projetos meus. A loja usa o storage para as fotos.",
    projects: [
      project({ ref: "qwhzkdlmrtyupoaszxcv", name: "loja-floriani", status: "ACTIVE_HEALTHY", lastActivityAt: ago(2 * HOUR), pause: { kind: "days", days: 7 } }),
      project({ ref: "plmoknijbuhvygctfxrd", name: "bot-discord", status: "ACTIVE_HEALTHY", lastActivityAt: ago(6 * 24 * HOUR), pause: { kind: "days", days: 1 } }),
    ],
  }),
  account({
    id: "a2",
    alias: "Cliente Ana",
    color: "#e9b44c",
    email: "contato.ana@exemplo.com",
    projects: [
      project({ ref: "zaqxswcdevfrbgtnhymj", name: "site-ana", status: "ACTIVE_HEALTHY", lastActivityAt: ago(4 * 24 * HOUR), pause: { kind: "days", days: 3 } }),
      project({ ref: "mnbvcxzlkjhgfdsapoiu", name: "agenda-ana", status: "INACTIVE", pause: { kind: "paused" }, usageChecked: false }),
    ],
  }),
  account({
    id: "a3",
    alias: "Conta 3",
    color: "#ef7b6b",
    sessionStatus: "none",
    browserOpen: true,
    hasPassword: false,
    lastSyncAt: null,
    organizations: [],
    projects: [],
  }),
  account({
    id: "a4",
    alias: "Antiga",
    color: "#2b2b2b",
    email: "antiga@exemplo.com",
    sessionStatus: "expired",
    projects: [
      project({ ref: "qazwsxedcrfvtgbyhnuj", name: "landing", status: "ACTIVE_HEALTHY", lastActivityAt: ago(5 * 24 * HOUR), pause: { kind: "days", days: 2 } }),
    ],
  }),
];

const overview: Overview = { accounts, totalFreeSlots: 1, suggestedAccountId: "a2" };

export function installDemo(empty: boolean) {
  mockIPC((cmd) => {
    switch (cmd) {
      case "list_accounts":
      case "sync_all":
        return empty ? { accounts: [], totalFreeSlots: 0, suggestedAccountId: null } : overview;
      case "sync_account":
      case "update_account":
        return accounts[0];
      case "start_account":
        return accounts[2];
      case "project_secrets":
        return {
          projectRef: "qwhzkdlmrtyupoaszxcv",
          name: "loja-floriani",
          region: "sa-east-1",
          status: "ACTIVE_HEALTHY",
          organizationName: "Org",
          plan: "free",
          apiUrl: "https://qwhzkdlmrtyupoaszxcv.supabase.co",
          dashboardUrl: "https://supabase.com/dashboard/project/qwhzkdlmrtyupoaszxcv",
          anonKey: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.demo-anon",
          publishableKey: "sb_publishable_demo",
          serviceRoleKey: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.demo-service",
          secretKey: null,
          jwtSecret: "demo-jwt-secret-demo-jwt-secret",
          dbHost: "db.qwhzkdlmrtyupoaszxcv.supabase.co",
          dbPort: 5432,
          dbName: "postgres",
          dbUser: "postgres",
          ipv6Only: true,
          dbPassword: null,
          directUrl: "postgresql://postgres:[YOUR-PASSWORD]@db.qwhzkdlmrtyupoaszxcv.supabase.co:5432/postgres",
          poolerTransactionUrl:
            "postgresql://postgres.qwhzkdlmrtyupoaszxcv:[YOUR-PASSWORD]@aws-0-sa-east-1.pooler.supabase.com:6543/postgres",
          poolerSessionUrl:
            "postgresql://postgres.qwhzkdlmrtyupoaszxcv:[YOUR-PASSWORD]@aws-0-sa-east-1.pooler.supabase.com:5432/postgres",
          accessToken: null,
          bundle: "# Supabase: loja-floriani (demonstração)",
        };
      case "reveal_secret":
        return "senha-demo";
      case "plugin:event|listen":
        return 1;
      default:
        return null;
    }
  });
}
