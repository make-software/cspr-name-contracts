use core::ops::DerefMut;
use odra::casper_types::U256;
use odra::module::Revertible;
use odra::prelude::*;
use odra::ContractRef;
use odra_modules::access::{AccessControl, Role, DEFAULT_ADMIN_ROLE};
use odra_modules::security::Pauseable;

use crate::data_structures::{ExpirableVoucher, NameMintInfo, RenewalVoucher, TokenRenewalInfo};
use crate::{
    contracts::name_token::NameTokenContractRef,
    data_structures::{NameTokenMetadata, TokenizationVoucher},
};

use super::resolver::ResolverContractRef;
use super::utils;

pub const CONTROLLER_ROLE: Role = [2u8; 32];
// Reminting should be possible after 5 days.
const PENDING_DELETE_PERIOD: u64 = 5 * 24 * 60 * 60 * 1000; // 5 days
const MAX_GRACE_PERIOD: u64 = 365 * 24 * 60 * 60 * 1000; // 365 days

/// Registrar smart contract. It handles the registration and expiration of name tokens.
#[odra::module(errors = RegistrarError)]
pub struct Registrar {
    name_token: External<NameTokenContractRef>,
    access_control: SubModule<AccessControl>,
    grace_period: Var<u64>,
    pauseable: SubModule<Pauseable>,
}

#[odra::module]
impl Registrar {
    delegate! {
        to self.access_control {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
        }

        to self.pauseable {
            fn is_paused(&self) -> bool;
        }
    }

    /// Initializes the registrar with the name token contract address.
    pub fn init(&mut self, name_token: Address) {
        if !name_token.is_contract() {
            self.revert(RegistrarError::NameTokenIsNotValid);
        }
        let caller = self.env().caller();

        // Set NameToken address.
        self.name_token.set(name_token);

        // Init grace period to 0.
        self.grace_period.set(0);

        // Setup roles.
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &caller);
        self.access_control
            .unchecked_grant_role(&CONTROLLER_ROLE, &caller);
    }

    /// Temporarily stops the contract.
    pub fn pause(&mut self) {
        self.assert_caller_is_admin();
        self.pauseable.pause();
    }

    /// Returns to normal operation.
    pub fn unpause(&mut self) {
        self.assert_caller_is_admin();
        self.pauseable.unpause();
    }

    /// Returns the grace period.
    pub fn grace_period(&self) -> u64 {
        self.grace_period.get().unwrap_or_revert(self)
    }

    /// Try to resolve a full domain name to an address.
    pub fn resolve(&self, full_domain: String) -> Option<Address> {
        let token_name = utils::extract_token_name(&full_domain)?;
        let token_hash = self.compute_token_id(&token_name);
        if !self.name_token.is_token_valid(token_hash) {
            return None;
        }

        match self.name_token.resolver(token_hash) {
            Some(address) => self.resolver(address).resolve(full_domain),
            None => None,
        }
    }

    // Public functions.

    /// Expire a list of tokens if they are expired.
    pub fn expire(&mut self, token_ids: Vec<U256>) {
        self.pauseable.require_not_paused();
        let block_time = self.env().get_block_time();
        let grace_period = self.grace_period();
        for token_id in token_ids {
            self.expire_single(token_id, block_time, grace_period);
        }
    }

    /// Admin only. Sets the grace period.
    pub fn set_grace_period(&mut self, period: u64) {
        self.assert_caller_is_admin();
        if period > MAX_GRACE_PERIOD {
            self.revert(RegistrarError::GracePeriodTooLong);
        }
        self.grace_period.set(period);
    }

    /// Admin only. Transfer ownership of a list of tokens.
    pub fn admin_transfer(&mut self, new_owner: Address, token_ids: Vec<U256>) {
        self.assert_caller_is_admin();
        self.name_token.admin_transfer(new_owner, token_ids);
    }

    /// Admin only. Burn a list of tokens.
    pub fn admin_burn(&mut self, token_ids: Vec<U256>) {
        self.assert_caller_is_admin();
        let name_token = self.name_token.deref_mut();
        for id in token_ids {
            name_token.burn(id);
        }
    }

    /// Admin only. Prolong the expiration date of a list of tokens.
    pub fn admin_prolong(&mut self, tokens: Vec<TokenRenewalInfo>) {
        self.assert_caller_is_admin();
        self.prolong(tokens);
    }

    /// Admin only. Register a list of tokens.
    pub fn admin_register(&mut self, names: Vec<NameMintInfo>) {
        self.assert_caller_is_admin();
        self.register(names);
    }

    /// Admin only. Prolong the expiration date of a list of tokens and register a list of tokens.
    pub fn admin_prolong_and_register(
        &mut self,
        renewal_tokens: Vec<TokenRenewalInfo>,
        new_tokens: Vec<NameMintInfo>,
    ) {
        self.assert_caller_is_admin();
        self.prolong(renewal_tokens);
        self.register(new_tokens);
    }

    /// Controller only. Prolong the expiration date of a list of tokens.
    pub fn controller_prolong(&mut self, voucher: RenewalVoucher) {
        self.pauseable.require_not_paused();
        self.assert_caller_is_controller();
        self.assert_voucher_not_expired(&voucher);
        self.prolong(voucher.tokens);
    }

    /// Controller only. Register a list of tokens.
    pub fn controller_register(&mut self, voucher: TokenizationVoucher) {
        self.pauseable.require_not_paused();
        self.assert_voucher_not_expired(&voucher);
        self.assert_caller_is_controller();
        self.register(voucher.names);
    }

    /// Controller only. Prolong the expiration date of a list of tokens and register a list of tokens.
    pub fn controller_prolong_and_register(
        &mut self,
        renewal_voucher: RenewalVoucher,
        tokenization_voucher: TokenizationVoucher,
    ) {
        self.pauseable.require_not_paused();
        self.assert_caller_is_controller();
        self.assert_voucher_not_expired(&renewal_voucher);
        self.assert_voucher_not_expired(&tokenization_voucher);
        self.prolong(renewal_voucher.tokens);
        self.register(tokenization_voucher.names);
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

    fn expire_single(&mut self, token_id: U256, block_time: u64, grace_period: u64) {
        let metadata = self.wrapped_metadata(token_id);
        let token_expiration = metadata.expiration().unwrap_or_revert(self);
        if self.is_token_expired(token_expiration, grace_period, block_time) {
            self.name_token.burn(token_id);
        }
    }

    #[inline]
    fn compute_token_id(&self, label: &str) -> U256 {
        let hash = self.env().hash(label);
        U256::from(hash)
    }

    #[inline]
    fn assert_token_expired(&self, token_expiration: u64, block_time: u64) {
        let grace_period = self.grace_period();
        if !self.is_token_expired(token_expiration, grace_period, block_time) {
            self.revert(RegistrarError::TokenNotExpired);
        }
    }

    #[inline]
    fn assert_voucher_not_expired<T: ExpirableVoucher>(&self, voucher: &T) {
        let block_time = self.env().get_block_time();
        if voucher.expiration_time() < block_time {
            self.revert(RegistrarError::VoucherExpired);
        }
    }

    #[inline]
    fn resolver(&self, address: Address) -> ResolverContractRef {
        ResolverContractRef::new(self.env(), address)
    }

    #[inline]
    fn wrapped_metadata(&self, token_id: U256) -> NameTokenMetadata {
        self.name_token
            .token_metadata(token_id)
            .try_into()
            .unwrap_or_revert(self)
    }

    #[inline]
    fn is_token_expired(&self, token_expiration: u64, grace_period: u64, block_time: u64) -> bool {
        block_time > token_expiration + grace_period + PENDING_DELETE_PERIOD
    }

    fn prolong(&mut self, tokens: Vec<TokenRenewalInfo>) {
        let block_time = self.env().get_block_time();
        for token in tokens {
            // verify the new expiration date is in the future
            self.assert_token_expires_in_future(token.token_expiration, block_time);
            // Compute token hash.
            let token_id = token.token_id;
            // get the token metadata
            let mut metadata = self.wrapped_metadata(token_id);
            // check if the time for the renewal does not elapsed
            let expiration = metadata.expiration().unwrap_or_revert(self);
            self.assert_in_renewal_period(expiration);
            metadata.set_expiration(token.token_expiration);

            self.name_token
                .set_token_metadata(token_id, metadata.to_vec());
        }
    }

    fn register(&mut self, names: Vec<NameMintInfo>) {
        let block_time = self.env().get_block_time();
        for info in names {
            self.assert_token_expires_in_future(info.token_expiration, block_time);

            // Compute token hash.
            let token_id = self.compute_token_id(&info.label);

            // Check if token already exists.
            let token_exists = self.name_token.token_exists(token_id);

            // If token exists and is expired and grace period is over, burn it.
            if token_exists {
                let metadata = self.wrapped_metadata(token_id);
                self.assert_token_expired(metadata.expiration().unwrap_or_revert(self), block_time);
                self.name_token.burn(token_id);
            }

            // Mint token.
            let metadata = NameTokenMetadata::with_resolver(
                &info.label,
                info.token_expiration,
                self.name_token.get_default_resolver(),
            );
            self.name_token
                .mint(info.owner, token_id, metadata.to_vec());
        }
    }
}

#[odra::odra_error]
pub enum RegistrarError {
    ExpirationDateInThePast = 1201,
    TokenNotExpired = 1202,
    GracePeriodExpired = 1203,
    VoucherExpired = 1204,
    TokenDoesNotExist = 1205,
    GracePeriodTooLong = 1206,
    NameTokenIsNotValid = 1207,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_structures::TokenRenewalInfo,
        test_context::{
            generate_token_id, TestContext, GRACE_PERIOD, INIT_TIME, TOKEN_EXPIRATION, TOKEN_NAME,
        },
    };
    use odra::host::{Deployer, HostRef};
    use odra_modules::{access::errors::Error as AccessControlError, cep95::Burn};

    #[test]
    fn deploy_fails_if_account_set_as_name_token() {
        let env = odra_test::env();
        let result = Registrar::try_deploy(
            &env,
            RegistrarInitArgs {
                name_token: env.get_account(1),
            },
        );
        assert!(result.is_err());
    }

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
    fn test_grace_period_too_long() {
        let mut ctx = TestContext::install_raw();
        let (env, reg) = (ctx.env, &mut ctx.registrar);
        let admin = ctx.admin;

        // When Admin sets too long grace period.
        env.set_caller(admin);
        let result = reg.try_set_grace_period(MAX_GRACE_PERIOD + 1);

        // Then it fails with error.
        assert_eq!(result, Err(RegistrarError::GracePeriodTooLong.into()));
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
            TOKEN_NAME,
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
            TOKEN_NAME,
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
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // Then token is minted.
        ctx.expect_name_is_registered(alice, TOKEN_NAME);
    }

    #[test]
    fn register_the_same_name_before_expiration() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And token not expired.
        ctx.advance_block_time(TOKEN_EXPIRATION / 2);

        // When Admin tries to register the same token again.
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_NAME,
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
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And token expired, but within grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + PENDING_DELETE_PERIOD - 1);

        // When Admin tries to register the same token again.
        let result = ctx.try_name_register(
            admin,
            alice,
            TOKEN_NAME,
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
        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And token expired, and grace period is over.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + PENDING_DELETE_PERIOD + 1);

        // When Admin tries to register the same token again.
        ctx.with_name_registered(admin, bob, TOKEN_NAME);

        // Then Alice's token is burned.
        let event: Burn = ctx.token.get_event(-3).unwrap();
        let expected = Burn {
            from: alice,
            token_id: generate_token_id(TOKEN_NAME),
        };
        assert_eq!(event, expected);

        // And Bob's token is minted.
        ctx.expect_name_is_registered(bob, TOKEN_NAME);
    }

    #[test]
    fn test_token_expiration_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And is after grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + PENDING_DELETE_PERIOD + 1);

        // When anyone tries to expire the token.
        ctx.with_name_expired(TOKEN_NAME);

        // Then token is burned.
        assert_eq!(ctx.token.balance_of(alice), U256::zero());
    }

    #[test]
    fn on_expiration_default_resolver_is_invalidated() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        let full_domain = format!("{}.cspr", TOKEN_NAME);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);
        // And a resolution for the token is set.
        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution(full_domain.clone(), Some(alice));
        assert_eq!(
            ctx.default_resolver.resolve(full_domain.clone()),
            Some(alice)
        );
        assert_eq!(
            ctx.token.resolver(generate_token_id(TOKEN_NAME)),
            Some(*ctx.default_resolver.address())
        );

        // And is after grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + PENDING_DELETE_PERIOD + 1);

        // When anyone tries to expire the token.
        ctx.with_name_expired(TOKEN_NAME);

        // Then the resolution is cleared.
        assert_eq!(ctx.default_resolver.resolve(full_domain), None);
    }

    #[test]
    fn test_multi_tokens_expiration_after_grace_period() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let tokens = vec!["t1", "t2", "t3", "t4", "t5"];

        // Given Alice has 5 tokens.
        ctx.with_multi_names_registered(admin, alice, tokens.clone());
        assert_eq!(ctx.token.balance_of(alice), U256::from(5));
        // And is after grace period.
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + PENDING_DELETE_PERIOD + 1);

        // When anyone tries to expire the tokens.
        ctx.with_names_expired(tokens);

        // Then all the tokens are burned.
        assert_eq!(ctx.token.balance_of(alice), U256::from(0));
    }

    #[test]
    fn test_admin_transfer() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        ctx.admin_transfer(bob, vec![TOKEN_NAME]);

        // Then Alice's token is transferred to Bob.
        assert_eq!(ctx.token.balance_of(alice), U256::from(0));
        assert_eq!(ctx.token.balance_of(bob), U256::from(1));
    }

    #[test]
    fn test_admin_transfer_clears_default_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);
        let full_domain = format!("{}.cspr", TOKEN_NAME);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And a resolution for the token is set
        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution(full_domain.clone(), Some(alice));
        assert_eq!(
            ctx.default_resolver.resolve(full_domain.clone()),
            Some(alice)
        );
        assert_eq!(
            ctx.token.resolver(generate_token_id(TOKEN_NAME)),
            Some(*ctx.default_resolver.address())
        );

        ctx.admin_transfer(bob, vec![TOKEN_NAME]);

        // Then the resolution is cleared.
        assert_eq!(ctx.default_resolver.resolve(full_domain), None);
    }

    #[test]
    fn test_admin_transfer_sets_default_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice, bob) = (ctx.admin, ctx.alice, ctx.bob);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And change the resolver
        let resolver =
            Address::new("hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a")
                .unwrap();
        ctx.set_caller(alice);
        ctx.token
            .set_resolver(generate_token_id(TOKEN_NAME), resolver);

        let json = ctx.token.token_metadata(generate_token_id(TOKEN_NAME));
        let actual_resolver = NameTokenMetadata::try_from(json)
            .unwrap()
            .resolver()
            .unwrap();
        assert_eq!(actual_resolver, Some(resolver));

        ctx.admin_transfer(bob, vec![TOKEN_NAME]);

        // Then the resolution is cleared.
        let json = ctx.token.token_metadata(generate_token_id(TOKEN_NAME));
        let actual_resolver = NameTokenMetadata::try_from(json)
            .unwrap()
            .resolver()
            .unwrap();
        assert_eq!(actual_resolver, Some(ctx.token.get_default_resolver()));
    }

    #[test]
    fn test_admin_burn() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        ctx.admin_burn(vec![TOKEN_NAME]);

        // Then Alice's token is burned.
        assert_eq!(ctx.token.balance_of(alice), U256::from(0));
    }

    #[test]
    fn test_admin_burn_clears_default_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let full_domain = format!("{}.cspr", TOKEN_NAME);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // And a resolution for the token is set
        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution(full_domain.clone(), Some(alice));
        assert_eq!(
            ctx.default_resolver.resolve(full_domain.clone()),
            Some(alice)
        );
        assert_eq!(
            ctx.token.resolver(generate_token_id(TOKEN_NAME)),
            Some(*ctx.default_resolver.address())
        );

        ctx.admin_burn(vec![TOKEN_NAME]);

        // Then the resolution is cleared.
        assert_eq!(ctx.default_resolver.resolve(full_domain), None);
    }

    #[test]
    fn renew_with_expired_voucher_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        // When Admin tries to renew the token.
        ctx.set_caller(admin);

        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION;
        let tokens = vec![TokenRenewalInfo::new(
            generate_token_id(TOKEN_NAME),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD - 1);
        let result = ctx.registrar.try_controller_prolong(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::VoucherExpired.into()));
    }

    #[test]
    fn renew_when_paused_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);
        ctx.set_caller(admin);
        ctx.registrar.pause();

        // When Admin tries to renew the token.
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION;
        let tokens = vec![TokenRenewalInfo::new(
            generate_token_id(TOKEN_NAME),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        let result = ctx.registrar.try_controller_prolong(voucher);

        // Then registration fails.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }

    #[test]
    fn test_renew() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD - 1);
        // When Admin tries to renew the token.
        let test_token_name = TOKEN_NAME;
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION + GRACE_PERIOD;
        let tokens = vec![TokenRenewalInfo::new(
            generate_token_id(test_token_name),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.set_caller(admin);
        ctx.registrar.controller_prolong(voucher);

        // Then token expiration is updated.
        let metadata = ctx.token.token_metadata(generate_token_id(test_token_name));
        let expected = NameTokenMetadata::with_resolver(
            TOKEN_NAME,
            INIT_TIME + 2 * TOKEN_EXPIRATION,
            *ctx.default_resolver.address(),
        );
        assert_eq!(metadata, expected.to_vec());
    }

    #[test]
    fn test_renew_after_grace_period_fails() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

        ctx.advance_block_time(TOKEN_EXPIRATION + GRACE_PERIOD + 1);
        // When Admin tries to renew the token.
        let test_token_name = TOKEN_NAME;
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION + GRACE_PERIOD + 1;
        let tokens = vec![TokenRenewalInfo::new(
            generate_token_id(test_token_name),
            token_expiration,
        )];
        let voucher = RenewalVoucher::new(tokens, voucher_expiration);
        ctx.set_caller(admin);
        let result = ctx.registrar.try_controller_prolong(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::GracePeriodExpired.into()));
    }

    #[test]
    fn resolve_with_invalid_domain() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);
        let full_domain = "invalid".to_string();

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);

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

    #[test]
    fn test_controller_register_fails_when_paused() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);
        ctx.set_caller(admin);
        ctx.registrar.pause();

        // When Admin tries to register the token.
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION;
        let names = vec![NameMintInfo::new(TOKEN_NAME, alice, token_expiration)];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);
        let result = ctx.registrar.try_controller_register(voucher);

        // Then registration fails.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }

    #[test]
    fn test_controller_register_and_prolong_fails_when_paused() {
        let mut ctx = TestContext::install_and_setup();
        let (admin, alice) = (ctx.admin, ctx.alice);

        // Given Alice has a token.
        ctx.with_name_registered(admin, alice, TOKEN_NAME);
        ctx.set_caller(admin);
        ctx.registrar.pause();

        // When Admin tries to register the token.
        let token_expiration = INIT_TIME + 2 * TOKEN_EXPIRATION;
        let voucher_expiration = INIT_TIME + TOKEN_EXPIRATION;
        let names = vec![NameMintInfo::new(TOKEN_NAME, alice, token_expiration)];
        let tokens = vec![TokenRenewalInfo::new(
            generate_token_id(TOKEN_NAME),
            token_expiration,
        )];
        let renewal_voucher = RenewalVoucher::new(tokens, voucher_expiration);
        let voucher = TokenizationVoucher::new(names, voucher_expiration);
        let result = ctx
            .registrar
            .try_controller_prolong_and_register(renewal_voucher, voucher);

        // Then registration fails.
        assert_eq!(
            result,
            Err(odra_modules::security::errors::Error::UnpausedRequired.into())
        );
    }
}
