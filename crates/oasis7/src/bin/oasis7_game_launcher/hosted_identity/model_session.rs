//! Reuse the hosted identity authority for model sessions, without exporting a cookie.
use super::{IdentityService, digest, now_ms, reject, service, store::StoreError};
use oasis7_model_provider::connection::Principal;

impl IdentityService {
    pub(crate) fn model_principal(
        &mut self,
        headers: &str,
        writing: bool,
    ) -> Result<Principal, StoreError> {
        let now = now_ms();
        if !self.clock.observe(now) {
            return Err(reject("clock_unhealthy"));
        }
        let token = service::cookie_value(headers, "__Host-oasis7_sid")
            .ok_or_else(|| reject("session_expired"))?;
        if writing {
            let csrf = digest(&self.config.session_key, "csrf", token);
            self.validate_json_write(headers, &csrf)?;
        }
        let (account, session) = self.session(Some(token), now)?;
        Ok(Principal {
            account_id: account.account_id,
            session_id: session.session_id,
            auth_ref: service::session_key(&self.config.session_key, token),
        })
    }
    pub(crate) fn model_session_current(&mut self, principal: &Principal) -> bool {
        let now = now_ms();
        if !self.clock.observe(now) {
            return false;
        }
        let epoch = self.config.rp.identity_instance_epoch;
        self.store
            .transaction(|tx| service::valid_session(tx, &principal.auth_ref, epoch, now))
            .is_ok_and(|(account, session)| {
                account.account_id == principal.account_id
                    && session.session_id == principal.session_id
            })
    }
}

#[cfg(test)]
#[path = "model_session_tests.rs"]
mod model_session_tests;
