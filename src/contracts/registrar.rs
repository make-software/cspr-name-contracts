use odra::prelude::*;
use odra::{args::Maybe, module::Module, Address, ContractRef, SubModule, UnwrapOrRevert, Var};
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};

use crate::{
    contracts::name_token::NameTokenContractRef,
    data_structures::{NameTokenMetadata, TokenizationVoucher},
};

pub const CONTROLLER_ROLE: Role = [2u8; 32];

#[odra::module]
pub struct Registrar {
    name_token: Var<Address>,
    access_control: SubModule<AccessControl>,
    grace_period: Var<u64>,
}

#[odra::module]
impl Registrar {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }
    }

    pub fn init(&mut self, name_token: Address) {
        let caller = self.env().caller();

        // Set NameToken address.
        self.name_token.set(name_token);

        // Init grace period to 0.
        self.grace_period.set(0);

        // Setup roles.
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &caller);
        self.access_control
            .set_admin_role(&CONTROLLER_ROLE, &DEFAULT_ADMIN_ROLE);

        // Consider removing this line.
        self.access_control
            .unchecked_grant_role(&CONTROLLER_ROLE, &caller);
    }

    // Getter functions.

    pub fn grace_period(&self) -> u64 {
        self.grace_period.get().unwrap_or_revert(&self.env())
    }

    // Admin functions.

    pub fn set_grace_period(&mut self, period: u64) {
        self.assert_caller_is_admin();
        self.grace_period.set(period);
    }

    pub fn admin_transfer(&mut self, new_owner: Address, token_hashes: Vec<String>) {
        self.assert_caller_is_admin();
        self.name_token().admin_transfer(new_owner, token_hashes);
    }

    pub fn admin_burn(&mut self, token_hashes: Vec<String>) {
        self.assert_caller_is_admin();
        let mut name_token = self.name_token();
        for token_hash in token_hashes {
            name_token.burn(Maybe::None, Maybe::Some(token_hash));
        }
    }

    // Controller functions.

    pub fn register(&mut self, voucher: TokenizationVoucher) {
        self.assert_caller_is_controller();
        self.verify_voucher(&voucher);

        // Compute token hash.
        let token_hash = self.compute_namehash(&voucher.label);

        // Check if token already exists.
        let mut name_token = self.name_token();
        let token_exists = name_token.token_exists(&token_hash);

        // If token exists and is expired and grace period is over, burn it.
        if token_exists {
            let metadata = name_token.metadata_by_hash(&token_hash);
            if metadata.expiration + self.grace_period() <= self.env().get_block_time() {
                name_token.burn(Maybe::None, Maybe::Some(token_hash.clone()));
            } else {
                self.env().revert(RegistrarError::TokenNotExpired);
            }
        }

        // Mint token.
        let metadata = NameTokenMetadata::from(&voucher);
        let metadata = metadata.to_json().unwrap_or_revert(&self.env());
        self.name_token()
            .mint(voucher.buyer, metadata, Maybe::Some(token_hash));
    }

    pub fn expire(&mut self, token_hashes: Vec<String>) {
        let block_time = self.env().get_block_time();
        let grace_period = self.grace_period();
        for token_hash in token_hashes {
            self.expire_single(token_hash, block_time, grace_period);
        }
    }

    pub fn renew(&mut self, token_hash: String, expiration: u64) {
        self.assert_caller_is_controller();
        let mut name_token = self.name_token();
        let metadata = name_token.metadata_by_hash(&token_hash);
        self.assert_in_grace_period(metadata.expiration);
        let new_metadata = NameTokenMetadata::new(&metadata.label, expiration);
        let new_metadata = new_metadata.to_json().unwrap_or_revert(&self.env());
        name_token.set_token_metadata(Maybe::None, Maybe::Some(token_hash), new_metadata);
    }
}

impl Registrar {
    pub fn assert_caller_is_controller(&self) {
        self.access_control
            .check_role(&CONTROLLER_ROLE, &self.env().caller());
    }

    pub fn assert_caller_is_admin(&self) {
        self.access_control
            .check_role(&DEFAULT_ADMIN_ROLE, &self.env().caller());
    }

    pub fn assert_in_grace_period(&self, expiration: u64) {
        let grace_period = self.grace_period();
        let block_time = self.env().get_block_time();
        if expiration + grace_period < block_time {
            self.env().revert(RegistrarError::GracePeriodExpired);
        }
    }

    pub fn name_token(&self) -> NameTokenContractRef {
        let env = self.env();
        let address = self.name_token.get().unwrap_or_revert(&env);
        NameTokenContractRef::new(env, address)
    }

    pub fn verify_voucher(&self, voucher: &TokenizationVoucher) {
        if voucher.expiration < self.env().get_block_time() {
            self.env().revert(RegistrarError::ExpirationDateInThePast);
        }
    }

    pub fn expire_single(&mut self, token_hash: String, block_time: u64, grace_period: u64) {
        let mut name_token = self.name_token();
        let metadata = name_token.metadata_by_hash(&token_hash);
        if metadata.expiration + grace_period < block_time {
            name_token.burn(Maybe::None, Maybe::Some(token_hash));
        }
    }

    pub fn compute_namehash(&self, label: &String) -> String {
        let hash = self.env().hash(label);
        hex::encode(hash)
    }
}

#[odra::odra_error]
pub enum RegistrarError {
    ExpirationDateInThePast = 1001,
    TokenNotExpired = 1002,
    GracePeriodExpired = 1003,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::{blake2b, TestContext, EXPIRATION, GRACE_PERIOD, INIT_TIME};
    use odra_modules::access::errors::Error as AccessControlError;

    #[test]
    fn test_admin_can_manage_controller_role() {
        let mut ctx = TestContext::install_raw();
        let (env, reg) = (ctx.env, &mut ctx.registrar);
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Alice has no role.
        assert!(!reg.has_role(&CONTROLLER_ROLE, &alice));

        // Alice can't grant roles.
        env.set_caller(alice);
        let result = reg.try_grant_role(&CONTROLLER_ROLE, &alice);
        assert_eq!(result.unwrap_err(), AccessControlError::MissingRole.into());

        // Admin can grant roles.
        env.set_caller(admin);
        reg.grant_role(&CONTROLLER_ROLE, &alice);
        assert!(reg.has_role(&CONTROLLER_ROLE, &alice));

        // Alice can't revoke roles.
        env.set_caller(alice);
        let result = reg.try_revoke_role(&CONTROLLER_ROLE, &alice);
        assert_eq!(result.unwrap_err(), AccessControlError::MissingRole.into());

        // Admin can revoke roles.
        env.set_caller(admin);
        reg.revoke_role(&CONTROLLER_ROLE, &alice);
        assert!(!reg.has_role(&CONTROLLER_ROLE, &alice));
    }

    #[test]
    fn test_admin_can_manage_grace_period() {
        let mut ctx = TestContext::install_raw();
        let (env, reg) = (ctx.env, &mut ctx.registrar);
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given the initial grace period is 0.
        assert_eq!(reg.grace_period(), 0);

        // When Admin sets grace period.
        env.set_caller(admin);
        reg.set_grace_period(100);

        // Then grace period is changed.
        assert_eq!(reg.grace_period(), 100);

        // When Alice tries to set grace period.
        env.set_caller(alice);
        let result = reg.try_set_grace_period(200);
        assert_eq!(result, Err(AccessControlError::MissingRole.into()));

        // Then grace period is not changed.
        assert_eq!(reg.grace_period(), 100);
    }

    #[test]
    fn register_with_past_expiration_time_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // When Admin try to register with expiration time in past.
        let expiration = INIT_TIME - 1;
        let result = ctx.try_name_register(&admin, &alice, "test", expiration);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::ExpirationDateInThePast.into()));
    }

    #[test]
    fn test_register_successful_mint() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // When Admin try to register with expiration time in future.
        ctx.with_name_registered(&admin, &alice, "test");

        // Then token is minted.
        ctx.expect_name_is_registered(&alice, "test");
    }

    #[test]
    fn register_the_same_name_before_expiration() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        // And token not expired.
        ctx.env.advance_block_time(EXPIRATION / 2);

        // When Admin trys to register the same token again.
        let result = ctx.try_name_register(&admin, &alice, "test", ctx.expiration_time());

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));
    }

    #[test]
    fn register_the_same_name_before_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        // And token expired, but within grace period.
        ctx.env.advance_block_time(EXPIRATION + GRACE_PERIOD / 2);

        // When Admin trys to register the same token again.
        let result = ctx.try_name_register(&admin, &alice, "test", ctx.expiration_time());

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));
    }

    #[test]
    fn register_the_same_name_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        // And token expired, and grace period is over.
        ctx.env.advance_block_time(EXPIRATION + GRACE_PERIOD + 1);

        // When Admin trys to register the same token again.
        ctx.with_name_registered(&admin, &bob, "test");

        // Then Alice's token is burned.
        // TODO: Add more checks.
        // let event: Burn = ctx.register.get_event(-1).unwrap();
        // let expected = Burn::new(ctx.user, token_id, ctx.operator);

        // And Bob's token is minted.
        ctx.expect_name_is_registered(&bob, "test");
    }

    // TODO: Check expiring multiple tokens.
    #[test]
    fn test_domain_expiration_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        // And is after grace period.
        ctx.env.advance_block_time(EXPIRATION + GRACE_PERIOD + 1);

        // When anyone tries to expire the token.
        ctx.with_name_expired("test");

        // Then token is burned.
        // TODO: Check if token is burned.
        // assert_eq!(ctx.name_token.balance_of(alice), 0);
    }

    #[test]
    fn test_admin_transfer() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        ctx.admin_transfer(&bob, vec!["test"]);

        // Then Alice's token is transferred to Bob.
        assert_eq!(ctx.token.balance_of(alice), 0);
        assert_eq!(ctx.token.balance_of(bob), 1);
    }

    #[test]
    fn test_admin_burn() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        ctx.admin_burn(vec!["test"]);

        // Then Alice's token is burned.
        assert_eq!(ctx.token.balance_of(alice), 0);
    }

    #[test]
    fn test_renew() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(&admin, &alice, "test");

        // When Admin tries to renew the token.
        let test_token_hash = blake2b("test");
        ctx.env.set_caller(admin);
        ctx.registrar
            .renew(test_token_hash.clone(), INIT_TIME + 200);

        // Then token expiration is updated.
        let metadata = ctx.token.metadata_by_hash(&test_token_hash);
        let expected = NameTokenMetadata::new("test", INIT_TIME + 200);
        assert_eq!(metadata, expected);
    }
}
