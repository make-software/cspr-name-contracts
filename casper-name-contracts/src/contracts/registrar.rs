use core::ops::DerefMut;

use odra::module::Revertible;
use odra::{args::Maybe, module::Module, Address, SubModule, UnwrapOrRevert, Var};
use odra::{prelude::*, External};
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};

use crate::data_structures::{ExpirableVoucher, RenewalVoucher};
use crate::{
    contracts::name_token::NameTokenContractRef,
    data_structures::{NameTokenMetadata, TokenizationVoucher},
};

use super::resolver::ResolverContractRef;
use super::utils;

pub const CONTROLLER_ROLE: Role = [2u8; 32];

#[odra::module]
pub struct Registrar {
    name_token: External<NameTokenContractRef>,
    access_control: SubModule<AccessControl>,
    grace_period: Var<u64>,
    default_resolver: External<ResolverContractRef>,
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

    pub fn init(&mut self, name_token: Address, default_resolver: Address) {
        let caller = self.env().caller();

        // Set NameToken address.
        self.name_token.set(name_token);
        // Set default resolver address.
        self.default_resolver.set(default_resolver);

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
        self.grace_period.get().unwrap_or_revert(self)
    }

    // Admin functions.

    pub fn set_grace_period(&mut self, period: u64) {
        self.assert_caller_is_admin();
        self.grace_period.set(period);
    }

    pub fn admin_transfer(&mut self, new_owner: Address, token_hashes: Vec<String>) {
        self.assert_caller_is_admin();
        self.name_token.admin_transfer(new_owner, token_hashes);
    }

    pub fn admin_burn(&mut self, token_hashes: Vec<String>) {
        self.assert_caller_is_admin();
        for token_hash in token_hashes {
            let metadata = self.name_token.metadata_by_hash(&token_hash);
            self.burn(&token_hash, &metadata);
        }
    }

    pub fn expire(&mut self, token_hashes: Vec<String>) {
        let block_time = self.env().get_block_time();
        let grace_period = self.grace_period();
        for token_hash in token_hashes {
            self.expire_single(token_hash, block_time, grace_period);
        }
    }

    // Controller functions.
    pub fn register(&mut self, voucher: TokenizationVoucher) {
        let block_time = self.env().get_block_time();
        self.assert_voucher_not_expired(&voucher, block_time);
        self.assert_caller_is_controller();
        for info in voucher.names {
            self.assert_token_expires_in_future(info.token_expiration, block_time);

            // Compute token hash.
            let token_hash = self.compute_namehash(&info.label);

            // Check if token already exists.
            let token_exists = self.name_token.token_exists(&token_hash);

            // If token exists and is expired and grace period is over, burn it.
            if token_exists {
                let metadata = self.name_token.metadata_by_hash(&token_hash);
                self.assert_token_expired(metadata.expiration, block_time);
                self.burn(&token_hash, &metadata);
            }

            // Mint token.
            let metadata = NameTokenMetadata::with_resolver(
                &info.label,
                info.token_expiration,
                *self.default_resolver.address(),
            );
            let metadata = metadata.to_json().unwrap_or_revert(self);
            mint(
                self.name_token.deref_mut(),
                info.owner,
                metadata,
                token_hash,
            );
        }
    }

    pub fn prolong(&mut self, voucher: RenewalVoucher) {
        self.assert_caller_is_controller();
        let block_time = self.env().get_block_time();
        self.assert_voucher_not_expired(&voucher, block_time);
        for token in voucher.tokens {
            // verify the new expiration date is in the future
            self.assert_token_expires_in_future(token.token_expiration, block_time);
            // Compute token hash.
            let token_hash = self.compute_namehash(&token.token_id);
            // get the token metadata
            let metadata = self.name_token.metadata_by_hash(&token_hash);
            // check if the time for the renewal does not elapsed
            self.assert_in_renewal_period(metadata.expiration);
            let new_metadata = NameTokenMetadata {
                expiration: token.token_expiration,
                ..metadata
            };
            let new_metadata = new_metadata.to_json().unwrap_or_revert(self);
            set_token_metadata(self.name_token.deref_mut(), token_hash, new_metadata);
        }
    }

    pub fn set_default_resolver(&mut self, resolver: Address) {
        self.assert_caller_is_admin();
        self.default_resolver.set(resolver);
    }

    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        let token_name = utils::extract_token_name(&full_domain)?;
        let token_hash = self.compute_namehash(&token_name);
        if !self.name_token.is_token_valid(&token_hash) {
            return None;
        }

        match self.name_token.resolver(token_hash) {
            Some(address) => self.resolver(address).resolve(full_domain),
            None => None,
        }
    }
}

impl Registrar {
    fn assert_caller_is_controller(&self) {
        self.access_control
            .check_role(&CONTROLLER_ROLE, &self.env().caller());
    }

    fn assert_caller_is_admin(&self) {
        self.access_control
            .check_role(&DEFAULT_ADMIN_ROLE, &self.env().caller());
    }

    fn assert_in_renewal_period(&mut self, expiration: u64) {
        let grace_period = self.grace_period();
        let block_time = self.env().get_block_time();
        if block_time > expiration + grace_period {
            self.revert(RegistrarError::GracePeriodExpired);
        }
    }

    fn assert_token_expires_in_future(&self, token_expiration: u64, block_time: u64) {
        if token_expiration < block_time {
            self.revert(RegistrarError::ExpirationDateInThePast);
        }
    }

    fn expire_single(&mut self, token_hash: String, block_time: u64, grace_period: u64) {
        let metadata = self.name_token.metadata_by_hash(&token_hash);
        if metadata.expiration + grace_period < block_time {
            self.burn(&token_hash, &metadata);
        }
    }

    fn compute_namehash(&self, label: &String) -> String {
        let hash = self.env().hash(label);
        utils::to_utf8_string(&hash).unwrap_or_revert(self)
    }

    #[inline]
    fn assert_token_expired(&self, token_expiration: u64, block_time: u64) {
        let rebuy_time = token_expiration + self.grace_period();
        if block_time < rebuy_time {
            self.revert(RegistrarError::TokenNotExpired);
        }
    }

    #[inline]
    fn assert_voucher_not_expired<T: ExpirableVoucher>(&self, voucher: &T, block_time: u64) {
        if voucher.expiration_time() < block_time {
            self.revert(RegistrarError::VoucherExpired);
        }
    }

    #[inline]
    fn burn(&mut self, token_hash: &str, metadata: &NameTokenMetadata) {
        let resolver = self.name_token.resolver(token_hash.to_owned());
        if let Some(resolver) = resolver {
            // Cleanup only default resolver.
            if resolver == *self.default_resolver.address() {
                self.resolver(resolver).cleanup(token_hash.to_owned());
            }
        }
        let metadata = NameTokenMetadata {
            resolver: None,
            ..metadata.clone()
        };
        let token_meta_data = metadata.to_json().unwrap_or_revert(self);

        let name_token = self.name_token.deref_mut();
        set_token_metadata(name_token, token_hash.to_owned(), token_meta_data);
        burn(name_token, token_hash.to_owned());
    }

    #[inline]
    fn resolver(&self, address: Address) -> ResolverContractRef {
        ResolverContractRef::new(self.env(), address)
    }
}

#[inline]
fn mint(
    name_token: &mut NameTokenContractRef,
    buyer: Address,
    metadata: String,
    token_hash: String,
) {
    name_token.mint(buyer, metadata, Maybe::Some(token_hash));
}

#[inline]
fn set_token_metadata(
    name_token: &mut NameTokenContractRef,
    token_hash: String,
    token_meta_data: String,
) {
    name_token.set_token_metadata(Maybe::None, Maybe::Some(token_hash), token_meta_data);
}

#[inline]
fn burn(name_token: &mut NameTokenContractRef, token_hash: String) {
    name_token.burn(Maybe::None, Maybe::Some(token_hash));
}

#[odra::odra_error]
pub enum RegistrarError {
    ExpirationDateInThePast = 1201,
    TokenNotExpired = 1202,
    GracePeriodExpired = 1203,
    VoucherExpired = 1204,
    TokenDoesNotExist = 1205,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_structures::TokenRenewalInfo,
        test_context::{
            blake2b, TestContext, GRACE_PERIOD, INIT_TIME, TOKEN_EXPIRATION, TOKEN_HASH,
        },
    };
    use odra::host::HostRef;
    use odra_modules::{access::errors::Error as AccessControlError, cep78::events::Burn};

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
        let token_expiration = INIT_TIME - 1;
        let voucher_expiration = ctx.voucher_expiration_time();
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_HASH,
            token_expiration,
            voucher_expiration,
        );

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::ExpirationDateInThePast.into()));
    }

    #[test]
    fn register_with_expired_voucher_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // When Admin try to register with expiration time in past.
        let token_expiration = ctx.token_expiration_time();
        let voucher_expiration = INIT_TIME - 1;
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_HASH,
            token_expiration,
            voucher_expiration,
        );

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::VoucherExpired.into()));
    }

    #[test]
    fn test_register_successful_mint() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // When Admin try to register with expiration time in future.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // Then token is minted.
        ctx.expect_name_is_registered(alice, TOKEN_HASH);
    }

    #[test]
    fn register_the_same_name_before_expiration() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // And token not expired.
        ctx.advance_block_time(TOKEN_EXPIRATION / 2);

        // When Admin tries to register the same token again.
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_HASH,
            ctx.token_expiration_time(),
            ctx.voucher_expiration_time(),
        );

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));
    }

    #[test]
    fn register_the_same_name_before_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // And token expired, but within grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD / 2);

        // When Admin tries to register the same token again.
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_HASH,
            ctx.token_expiration_time(),
            ctx.voucher_expiration_time(),
        );

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));
    }

    #[test]
    fn register_the_same_name_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);
        let registrar_address = *ctx.registrar.address();
        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // And token expired, and grace period is over.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + 1);

        // When Admin tries to register the same token again.
        ctx.with_name_registered(admin, bob, TOKEN_HASH);

        // Then Alice's token is burned.
        let event: Burn = ctx.token.get_event(-2).unwrap();
        let expected = Burn::new(alice, blake2b(TOKEN_HASH), registrar_address);
        assert_eq!(event, expected);

        // And Bob's token is minted.
        ctx.expect_name_is_registered(bob, TOKEN_HASH);
    }

    #[test]
    fn test_token_expiration_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // And is after grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + 1);

        // When anyone tries to expire the token.
        ctx.with_name_expired(TOKEN_HASH);

        // Then token is burned.
        assert_eq!(ctx.token.balance_of(alice), 0);
    }

    #[test]
    fn test_multi_tokens_expiration_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let tokens = vec!["t1", "t2", "t3", "t4", "t5"];

        // Given Alice has 5 tokens.
        ctx.with_multi_names_registered(admin, alice, tokens.clone());
        assert_eq!(ctx.token.balance_of(alice), 5);
        // And is after grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + 1);

        // When anyone tries to expire the tokens.
        ctx.with_names_expired(tokens);

        // Then all the tokens are burned.
        assert_eq!(ctx.token.balance_of(alice), 0);
    }

    #[test]
    fn test_admin_transfer() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        ctx.admin_transfer(bob, vec![TOKEN_HASH]);

        // Then Alice's token is transferred to Bob.
        assert_eq!(ctx.token.balance_of(alice), 0);
        assert_eq!(ctx.token.balance_of(bob), 1);
    }

    #[test]
    fn test_admin_burn() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        ctx.admin_burn(vec![TOKEN_HASH]);

        // Then Alice's token is burned.
        assert_eq!(ctx.token.balance_of(alice), 0);
    }

    #[test]
    fn renew_with_expired_voucher_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // When Admin tries to renew the token.
        ctx.set_caller(admin);

        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION;
        let tokens = vec![TokenRenewalInfo::new(blake2b(TOKEN_HASH), token_expiration)];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD - 1);
        let result = ctx.registrar.try_prolong(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::VoucherExpired.into()));
    }

    #[test]
    fn test_renew() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD - 1);
        // When Admin tries to renew the token.
        let test_token_hash = TOKEN_HASH;
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION + GRACE_PERIOD;
        let tokens = vec![TokenRenewalInfo::new(
            test_token_hash.to_owned(),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.set_caller(admin);
        ctx.registrar.prolong(voucher);

        // Then token expiration is updated.
        let metadata = ctx.token.metadata_by_hash(&blake2b(test_token_hash));
        let expected = NameTokenMetadata::with_resolver(
            TOKEN_HASH,
            INIT_TIME + 2 * TOKEN_EXPIRATION,
            *ctx.default_resolver.address(),
        );
        assert_eq!(metadata, expected);
    }

    #[test]
    fn test_renew_after_grace_period_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + 1);
        // When Admin tries to renew the token.
        let test_token_hash = TOKEN_HASH;
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION + GRACE_PERIOD + 1;
        let tokens = vec![TokenRenewalInfo::new(
            test_token_hash.to_string(),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.set_caller(admin);
        let result = ctx.registrar.try_prolong(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::GracePeriodExpired.into()));
    }

    #[test]
    fn test_set_default_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, resolver) = (ctx.admin, ctx.alice, *ctx.default_resolver.address());

        // When Admin sets default resolver.
        ctx.set_caller(admin);
        let result = ctx.registrar.try_set_default_resolver(resolver);
        // Then the operation succeeds.
        assert_eq!(result, Ok(()));

        // When Admin sets default resolver.
        ctx.set_caller(alice);
        let result = ctx.registrar.try_set_default_resolver(resolver);
        // Then the operation fails.
        assert!(result.is_err());
    }

    #[test]
    fn resolve_with_invalid_domain() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let full_domain = "invalid".to_string();

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_HASH);

        // When anyone tries to resolve an invalid domain.
        let result = ctx.registrar.resolve(full_domain);

        // Then the result is None.
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_with_invalid_token() {
        let ctx = TestContext::install_and_setup();
        let full_domain = "odra.cspr".to_string();

        // When anyone tries to resolve an invalid domain.
        let result = ctx.registrar.resolve(full_domain);

        // Then the result is None.
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_with_valid_domain() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let full_domain = "odra.cspr".to_string();

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, "odra");

        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution(full_domain.clone(), Some(alice));

        // When anyone tries to resolve a valid domain.
        let result = ctx.registrar.resolve(full_domain);

        // Then the result is the token owner.
        assert_eq!(result, Some(alice));
    }
}
