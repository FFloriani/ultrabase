//! Regras do plano grátis, sem banco nem rede, para dar para testar sozinhas.

use chrono::{DateTime, Utc};

use crate::models::PauseState;

/// 2 projetos grátis por conta, somando todas as organizações onde ela é dona ou admin.
/// Fonte: https://supabase.com/docs/guides/platform/billing-on-supabase (lida em 09/11/2026).
pub const FREE_PROJECT_LIMIT: u32 = 2;

/// Projeto grátis pausa depois de 1 semana sem uso. Fonte: https://supabase.com/pricing
pub const PAUSE_AFTER_DAYS: i64 = 7;

/// Plano desconhecido conta como grátis: é melhor prometer uma vaga a menos do que uma a mais.
pub fn is_free_plan(plan: Option<&str>) -> bool {
    plan.is_none_or(|p| p.eq_ignore_ascii_case("free"))
}

/// Projeto pausado não ocupa vaga. Qualquer outro status (até "pausando") ainda ocupa.
pub fn counts_toward_limit(status: &str, plan: Option<&str>) -> bool {
    is_free_plan(plan) && !matches!(status, "INACTIVE" | "REMOVED")
}

pub fn slots_free(used: u32) -> u32 {
    FREE_PROJECT_LIMIT.saturating_sub(used)
}

pub fn pause_state(
    status: &str,
    plan: Option<&str>,
    usage_checked: bool,
    last_activity: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> PauseState {
    if status == "INACTIVE" {
        return PauseState::Paused;
    }
    if !is_free_plan(plan) {
        return PauseState::Paid;
    }
    if !usage_checked {
        return PauseState::Unknown;
    }
    let days = match last_activity {
        None => 0,
        Some(at) => (PAUSE_AFTER_DAYS - (now - at).num_days()).clamp(0, PAUSE_AFTER_DAYS),
    };
    PauseState::Days { days }
}

pub struct SlotCandidate<'a> {
    pub id: &'a str,
    pub token_ok: bool,
    pub slots_free: u32,
    pub project_count: usize,
}

/// Conta para o próximo projeto: token funcionando, mais vagas livres, menos projetos.
/// Empate fica com a que vem primeiro na lista.
pub fn suggest_account<'a>(candidates: &[SlotCandidate<'a>]) -> Option<&'a str> {
    candidates
        .iter()
        .filter(|c| c.token_ok && c.slots_free > 0)
        .min_by_key(|c| (std::cmp::Reverse(c.slots_free), c.project_count))
        .map(|c| c.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, 12, 0, 0).unwrap()
    }

    #[test]
    fn paused_project_frees_slot() {
        assert!(!counts_toward_limit("INACTIVE", Some("free")));
        assert!(counts_toward_limit("ACTIVE_HEALTHY", Some("free")));
        assert!(counts_toward_limit("PAUSING", Some("free")));
    }

    #[test]
    fn paid_org_does_not_use_free_slots() {
        assert!(!counts_toward_limit("ACTIVE_HEALTHY", Some("pro")));
    }

    #[test]
    fn unknown_plan_counts_as_free() {
        assert!(counts_toward_limit("ACTIVE_HEALTHY", None));
    }

    #[test]
    fn free_slots_never_negative() {
        assert_eq!(slots_free(0), 2);
        assert_eq!(slots_free(2), 0);
        assert_eq!(slots_free(5), 0);
    }

    #[test]
    fn pause_estimate_from_last_activity() {
        let st = |last| pause_state("ACTIVE_HEALTHY", Some("free"), true, last, now());
        assert_eq!(st(Some(now() - Duration::hours(1))), PauseState::Days { days: 7 });
        assert_eq!(st(Some(now() - Duration::hours(60))), PauseState::Days { days: 5 });
        assert_eq!(st(Some(now() - Duration::days(9))), PauseState::Days { days: 0 });
        assert_eq!(st(None), PauseState::Days { days: 0 });
    }

    #[test]
    fn pause_state_special_cases() {
        assert_eq!(pause_state("INACTIVE", Some("free"), true, None, now()), PauseState::Paused);
        assert_eq!(pause_state("ACTIVE_HEALTHY", Some("pro"), true, None, now()), PauseState::Paid);
        assert_eq!(pause_state("ACTIVE_HEALTHY", Some("free"), false, None, now()), PauseState::Unknown);
    }

    #[test]
    fn suggestion_prefers_most_free_slots_then_fewest_projects() {
        let c = |id, token_ok, slots_free, project_count| SlotCandidate { id, token_ok, slots_free, project_count };
        let list = [c("a", true, 1, 3), c("b", true, 2, 1), c("c", true, 2, 0), c("d", false, 2, 0)];
        assert_eq!(suggest_account(&list), Some("c"));
        assert_eq!(suggest_account(&[c("x", true, 0, 2), c("y", false, 2, 0)]), None);
    }
}
