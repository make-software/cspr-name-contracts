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
        self.access_control
            .unchecked_grant_role(&CONTROLLER_ROLE, &caller);
    }

    pub fn set_grace_period(&mut self, period: u64) {
        self.assert_caller_is_controller();
        self.grace_period.set(period);
    }

    pub fn grace_period(&self) -> u64 {
        self.grace_period.get().unwrap_or_revert(&self.env())
    }

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
            let current_metadata =
                name_token.metadata(Maybe::None, Maybe::Some(token_hash.clone()));
            let current_metadata =
                NameTokenMetadata::from_json(&current_metadata).unwrap_or_revert(&self.env());
            let grace_period = self.grace_period();
            let block_time = self.env().get_block_time();
            if current_metadata.expiration + grace_period <= block_time {
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

    pub fn admin_transfer(&mut self, new_owner: Address, token_hashes: Vec<String>) {
        self.assert_caller_is_admin();
        self.name_token().admin_transfer(new_owner, token_hashes);
    }

    pub fn compute_namehash(&self, label: &String) -> String {
        let hash = self.env().hash(label);
        hex::encode(hash)
    }

    pub fn admin_burn(&mut self, token_hashes: Vec<String>) {
        self.assert_caller_is_controller();
        let mut name_token = self.name_token();
        for token_hash in token_hashes {
            name_token.burn(Maybe::None, Maybe::Some(token_hash));
        }
    }

    pub fn renew(&mut self, token_hash: String, expiration: u64) {
        self.assert_caller_is_controller();
        let mut name_token = self.name_token();
        let metadata = name_token.metadata(Maybe::None, Maybe::Some(token_hash.clone()));
        let metadata = NameTokenMetadata::from_json(&metadata).unwrap_or_revert(&self.env());
        let grace_period = self.grace_period();
        let block_time = self.env().get_block_time();
        if metadata.expiration + grace_period < block_time {
            self.env().revert(RegistrarError::GracePeriodExpired);
        }
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
        let metadata = name_token.metadata(Maybe::None, Maybe::Some(token_hash.clone()));
        let metadata = NameTokenMetadata::from_json(&metadata).unwrap_or_revert(&self.env());

        if metadata.expiration + grace_period < block_time {
            name_token.burn(Maybe::None, Maybe::Some(token_hash));
        }
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
    use crate::test_context::{TestContext, INIT_TIME};
    use odra_modules::access::errors::Error as AccessControlError;

    #[test]
    fn test_admin_can_manage_controller_role() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

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
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

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
    fn buy_with_fiat_currency() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;
        let token = &mut contracts.name_token;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

        // When Admin try to register with expiration time in past.
        env.set_caller(admin);
        let voucher = TokenizationVoucher::new("test", INIT_TIME - 1, alice);
        let result = reg.try_register(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::ExpirationDateInThePast.into()));

        // When Admin try to register with expiration time in future.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 1, alice);
        reg.register(voucher);

        // Then token is minted.
        let token_hash = reg.compute_namehash(&String::from("test"));
        assert_eq!(token.balance_of(alice), 1);
        assert_eq!(
            token.owner_of(Maybe::None, Maybe::Some(token_hash.clone())),
            alice
        );
        let metadata = token.metadata(Maybe::None, Maybe::Some(token_hash.clone()));
        let metadata = NameTokenMetadata::from_json(&metadata).unwrap();
        let expected = NameTokenMetadata::new("test", INIT_TIME + 1);
        assert_eq!(metadata, expected);

        // When Admin trys to register the same token again before expiration.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 2, alice);
        let result = reg.try_register(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));

        // When Admin trys to register the same token again after expiration,
        // but within grace period.
        env.set_caller(admin);
        reg.set_grace_period(100);
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 2, alice);
        let result = reg.try_register(voucher);

        // Then registration fails.
        assert_eq!(result, Err(RegistrarError::TokenNotExpired.into()));

        // When Admin trys to register the same token again after expiration,
        // and after grace period to the same user.
        env.advance_block_time(101);
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 103, alice);
        reg.register(voucher);

        // Then token is minted.
        assert_eq!(token.balance_of(alice), 1);
        assert_eq!(
            token.owner_of(Maybe::None, Maybe::Some(token_hash.clone())),
            alice
        );
        let metadata = token.metadata_by_hash(token_hash);
        let metadata = NameTokenMetadata::from_json(&metadata).unwrap();
        let expected = NameTokenMetadata::new("test", INIT_TIME + 103);
        assert_eq!(metadata, expected);

        // When Admin trys to register the same token again after expiration,
        // and after grace period to a different user.
        let bob = env.get_account(2);
        env.advance_block_time(1000);
        let voucher = TokenizationVoucher::new("test", env.block_time() + 500, bob);
        reg.register(voucher);

        // Then Alice's token is burned.
        assert_eq!(token.balance_of(alice), 0);

        // Then Bob's token is minted.
        assert_eq!(token.balance_of(bob), 1);

        // println!("{}", env.gas_report());
        // assert!(false);
    }

    #[test]
    fn test_domain_expiration() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;
        let token = &mut contracts.name_token;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

        // Given a token with expiration time in 100ms.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 100, alice);
        reg.register(voucher);

        // And a token with expiration time in 200ms.
        let voucher = TokenizationVoucher::new("test2", INIT_TIME + 200, alice);
        reg.register(voucher);

        // And time as 150ms.
        env.advance_block_time(150);

        // When anyone tries to expire both tokens.
        let test_token_hash = reg.compute_namehash(&String::from("test"));
        let test2_token_hash = reg.compute_namehash(&String::from("test2"));
        env.set_caller(env.get_account(2));
        reg.expire(vec![test_token_hash, test2_token_hash]);

        // Then only one token is burned.
        assert_eq!(token.balance_of(alice), 1);
    }

    #[test]
    fn test_admin_transfer() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;
        let token = &mut contracts.name_token;

        let admin = env.get_account(0);
        let alice = env.get_account(1);
        let bob = env.get_account(2);

        // Given Alice has a token.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 100, alice);
        reg.register(voucher);

        let test_token_hash = reg.compute_namehash(&String::from("test"));
        reg.admin_transfer(bob, vec![test_token_hash.clone()]);

        // Then Alice's token is transferred to Bob.
        assert_eq!(token.balance_of(alice), 0);
        assert_eq!(token.balance_of(bob), 1);
        assert_eq!(
            token.owner_of(Maybe::None, Maybe::Some(test_token_hash)),
            bob
        );
    }

    #[test]
    fn test_admin_burn() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;
        let token = &mut contracts.name_token;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

        // Given Alice has a token.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 100, alice);
        reg.register(voucher);

        let test_token_hash = reg.compute_namehash(&String::from("test"));
        reg.admin_burn(vec![test_token_hash.clone()]);

        // Then Alice's token is burned.
        assert_eq!(token.balance_of(alice), 0);
    }

    #[test]
    fn test_renew() {
        let env = odra_test::env();
        let mut contracts = TestContext::install(&env);
        let reg = &mut contracts.registrar;
        let token = &mut contracts.name_token;

        let admin = env.get_account(0);
        let alice = env.get_account(1);

        // Given Alice has a token.
        let voucher = TokenizationVoucher::new("test", INIT_TIME + 100, alice);
        reg.register(voucher);

        let test_token_hash = reg.compute_namehash(&String::from("test"));

        // When Admin tries to renew the token.
        env.set_caller(admin);
        reg.renew(test_token_hash.clone(), INIT_TIME + 200);

        // Then token expiration is updated.
        let metadata = token.metadata_by_hash(test_token_hash);
        let metadata = NameTokenMetadata::from_json(&metadata).unwrap();
        let expected = NameTokenMetadata::new("test", INIT_TIME + 200);
        assert_eq!(metadata, expected);
    }
}
