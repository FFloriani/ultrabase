export type TokenStatus = "missing" | "unchecked" | "ok" | "invalid";
/** Login do Supabase no navegador da conta. */
export type SessionStatus = "none" | "connected" | "expired";
export type LoginMethod = "email" | "github";

export type PauseState =
  | { kind: "paused" }
  | { kind: "paid" }
  | { kind: "unknown" }
  | { kind: "days"; days: number };

export interface Organization {
  id: string;
  slug: string;
  name: string;
  plan: string | null;
}

export interface ProjectView {
  ref: string;
  name: string;
  region: string;
  organizationId: string;
  status: string;
  createdAt: string | null;
  lastActivityAt: string | null;
  usageChecked: boolean;
  organizationName: string | null;
  plan: string | null;
  countsTowardLimit: boolean;
  pause: PauseState;
  apiUrl: string;
}

export interface AccountView {
  id: string;
  alias: string;
  color: string;
  email: string;
  loginMethod: LoginMethod;
  notes: string;
  username: string | null;
  sessionStatus: SessionStatus;
  tokenStatus: TokenStatus;
  lastSyncAt: string | null;
  lastSyncError: string | null;
  createdAt: string;
  hasPassword: boolean;
  hasToken: boolean;
  organizations: Organization[];
  projects: ProjectView[];
  slotLimit: number;
  slotsUsed: number;
  slotsFree: number;
  browserOpen: boolean;
}

export interface Overview {
  accounts: AccountView[];
  totalFreeSlots: number;
  suggestedAccountId: string | null;
}

/** password/token: undefined ou null = não mexe; "" = apaga; texto = grava. */
export interface AccountInput {
  alias: string;
  color: string;
  email: string;
  loginMethod: LoginMethod;
  notes: string;
  password?: string | null;
  token?: string | null;
}

/** Tudo de um projeto, com os segredos. `bundle` é o texto pronto para a IA da IDE. */
export interface ProjectSecrets {
  projectRef: string;
  name: string;
  region: string;
  status: string;
  organizationName: string | null;
  plan: string | null;
  apiUrl: string;
  dashboardUrl: string;
  anonKey: string | null;
  publishableKey: string | null;
  serviceRoleKey: string | null;
  secretKey: string | null;
  jwtSecret: string | null;
  dbHost: string;
  dbPort: number;
  dbName: string;
  dbUser: string;
  ipv6Only: boolean;
  dbPassword: string | null;
  directUrl: string;
  poolerTransactionUrl: string | null;
  poolerSessionUrl: string | null;
  accessToken: string | null;
  bundle: string;
}

export type BrowserTarget = "dashboard" | "tokens" | "signin" | "signup" | "project";
