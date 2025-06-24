#![allow(unused_variables)]
use crate::data_structures::NameTokenMetadata;
use odra::casper_types::bytesrepr::Bytes;
use odra::casper_types::U256;
use odra::module::Revertible;
use odra::{prelude::*, ContractRef};
use odra_modules::access::Ownable2Step;
use odra_modules::cep95::{CEP95Interface, Cep95, Error as Cep95Error};

use super::resolver::ResolverContractRef;

/// NameToken contract. It is a CEP95 token with additional functionalities.
#[odra::module(errors = NameTokenError)]
pub struct NameToken {
    token: SubModule<Cep95>,
    ownable: SubModule<Ownable2Step>,
    default_resolver: External<ResolverContractRef>,
    max_supply: Var<u64>,
    minted_tokens_count: Var<u64>,
    whitelist: Mapping<Address, bool>,
}

#[odra::module]
impl NameToken {
    delegate! {
        to self.token {
            fn name(&self) -> String;
            fn symbol(&self) -> String;
            fn balance_of(&self, owner: Address) -> U256;
            fn owner_of(&self, token_id: U256) -> Option<Address>;
            fn safe_transfer_from(&mut self, from: Address, to: Address, token_id: U256, data: Option<Bytes>);
            fn approve(&mut self, spender: Address, token_id: U256);
            fn revoke_approval(&mut self, token_id: U256);
            fn approved_for(&self, token_id: U256) -> Option<Address>;
            fn approve_for_all(&mut self, operator: Address);
            fn revoke_approval_for_all(&mut self, operator: Address);
            fn is_approved_for_all(&self, owner: Address, operator: Address) -> bool;
            fn token_metadata(&self, token_id: U256) -> Vec<(String, String)>;
        }

        to self.ownable {
            fn get_owner(&self) -> Address;
            fn get_pending_owner(&self) -> Option<Address>;
            fn transfer_ownership(&mut self, new_owner: &Address);
            fn accept_ownership(&mut self);
            fn renounce_ownership(&mut self);
        }
    }

    /// Initializes CEP95 with the given name and symbol.
    pub fn init(&mut self, name: String, symbol: String, max_supply: u64) {
        let caller = self.env().caller();

        self.token.symbol.set(symbol);
        self.token.name.set(name);
        self.max_supply.set(max_supply);
        self.ownable.init(caller);
    }

    pub fn token_exists(&self, token_id: U256) -> bool {
        self.token.exists(&token_id)
    }

    pub fn mint(
        &mut self,
        recipient: Address,
        token_id: U256,
        token_metadata: Vec<(String, String)>,
    ) {
        let caller = self.env().caller();
        self.assert_whitelisted(&caller);

        let minted_tokens_count = self.minted_tokens_count.get_or_default();
        if minted_tokens_count >= self.max_supply.get_or_default() {
            self.revert(NameTokenError::TokenSupplyDepleted);
        }
        if self.token.exists(&token_id) {
            self.revert(NameTokenError::InvalidTokenIdentifier);
        }
        // mint the token
        self.token.mint(recipient, token_id, token_metadata);
        // increment the minted tokens count
        self.minted_tokens_count.set(minted_tokens_count + 1);
    }

    pub fn burn(&mut self, token_id: U256) {
        let caller = self.env().caller();
        self.assert_whitelisted(&caller);

        // invalidate resolutions if the resolver is the default resolver and update metadata
        let mut metadata = self.wrapped_metadata(token_id);
        if let Some(resolver) = metadata.resolver().unwrap_or_revert(self) {
            if &resolver == self.default_resolver.address() {
                self.default_resolver.invalidate_resolutions(token_id);
            }
        }
        metadata.clear_resolver();
        self.set_token_metadata(token_id, metadata.to_vec());

        // burn the token
        self.token.burn(token_id);
    }

    pub fn admin_transfer(&mut self, recipient: Address, token_ids: Vec<U256>) {
        let caller = self.env().caller();
        self.assert_whitelisted(&caller);

        for token_id in token_ids {
            if !self.is_token_valid(token_id) {
                self.revert(NameTokenError::ExpiredTokenTransfer);
            }

            let owner = self
                .token
                .owner_of(token_id)
                .unwrap_or_revert_with(self, Cep95Error::ValueNotSet);
            self.token.raw_transfer_from(owner, recipient, token_id);
            // if called by an operator
            if caller != owner {
                self.cleanup(token_id);
            }
        }
    }

    pub fn transfer_from(&mut self, from: Address, to: Address, token_id: U256) {
        if !self.is_token_valid(token_id) {
            self.revert(NameTokenError::ExpiredTokenTransfer);
        }

        let caller = self.env().caller();
        let owner = self
            .token
            .owner_of(token_id)
            .unwrap_or_revert_with(self, Cep95Error::ValueNotSet);
        // if called by an operator
        self.token.transfer_from(from, to, token_id);
        if caller != owner {
            self.cleanup(token_id);
        }
    }

    pub fn set_token_metadata(&mut self, token_id: U256, token_metadata: Vec<(String, String)>) {
        let caller = self.env().caller();
        self.assert_whitelisted(&caller);
        self.token.set_metadata(token_id, token_metadata);
    }

    pub fn resolver(&self, token_id: U256) -> Option<Address> {
        let metadata: NameTokenMetadata = self.wrapped_metadata(token_id);
        metadata.resolver().unwrap_or_revert(self)
    }

    pub fn set_resolver(&mut self, token_id: U256, resolver: Address) {
        if self.token.owner_of(token_id) != Some(self.env().caller()) {
            self.revert(NameTokenError::InvalidTokenOwner);
        }
        let mut metadata: NameTokenMetadata = self.wrapped_metadata(token_id);
        metadata.set_resolver(resolver);
        self.token.set_metadata(token_id, metadata.to_vec());
    }

    pub fn assert_is_owner(&self, token_id: U256, address: Address) {
        let owner = self.token.owner_of(token_id);
        if owner != Some(address) {
            self.revert(NameTokenError::InvalidTokenOwner);
        }
    }

    pub fn is_token_valid(&self, token_id: U256) -> bool {
        if !self.token.exists(&token_id) {
            return false;
        }

        let metadata: NameTokenMetadata = self.wrapped_metadata(token_id);
        if metadata.expiration().unwrap_or_revert(self) < self.env().get_block_time() {
            return false;
        }
        true
    }

    /// Only admin. Set the default resolver.
    pub fn set_default_resolver(&mut self, resolver: Address) {
        let caller = self.env().caller();
        self.assert_whitelisted(&caller);
        if !resolver.is_contract() {
            self.revert(NameTokenError::InvalidResolver);
        }
        self.default_resolver.set(resolver);
    }

    /// Get the default resolver.
    pub fn get_default_resolver(&self) -> Address {
        *self.default_resolver.address()
    }

    pub fn whitelist(&mut self, address: Address) {
        let caller = self.env().caller();
        self.ownable.assert_owner(&caller);
        if self.whitelist.get(&address).unwrap_or_default() {
            self.revert(NameTokenError::WhitelistedAlready);
        }
        self.whitelist.set(&address, true);
    }

    pub fn revoke_whitelist(&mut self, address: Address) {
        let caller = self.env().caller();
        self.ownable.assert_owner(&caller);
        if !self.whitelist.get(&address).unwrap_or_default() {
            self.revert(NameTokenError::NotWhitelisted);
        }
        self.whitelist.set(&address, false);
    }
}

impl NameToken {
    #[inline]
    pub fn wrapped_metadata(&self, token_id: U256) -> NameTokenMetadata {
        let metadata = self.token.token_metadata(token_id);
        NameTokenMetadata::try_from(metadata).unwrap_or_revert(self)
    }

    #[inline]
    fn is_whitelisted(&self, address: &Address) -> bool {
        self.whitelist.get(address).unwrap_or_default()
    }

    #[inline]
    fn assert_whitelisted(&self, address: &Address) {
        if !self.is_whitelisted(address) {
            self.revert(NameTokenError::NotWhitelisted);
        }
    }

    fn cleanup(&mut self, token_id: U256) {
        let mut metadata = self.wrapped_metadata(token_id);
        let resolver = metadata.resolver().unwrap_or_revert(self);
        let default_resolver_address = *self.default_resolver.address();
        if resolver != Some(default_resolver_address) {
            metadata.set_resolver(default_resolver_address);
            self.token.set_metadata(token_id, metadata.to_vec());
        }
        self.default_resolver.invalidate_resolutions(token_id);
    }
}

#[odra::odra_error]
pub enum NameTokenError {
    NotWhitelisted = 1301,
    InvalidTokenOwner = 1302,
    ExpiredTokenTransfer = 1303,
    InvalidTokenIdentifier = 1304,
    InvalidResolver = 1305,
    TokenSupplyDepleted = 1306,
    WhitelistedAlready = 1307,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::{generate_token_id, TestContext, INIT_TIME, TOKEN_EXPIRATION};

    #[test]
    fn test_supply_depletion() {
        // Given a token with max supply of 10
        let max_supply = 10u64;
        let mut ctx = TestContext::install_raw_with_supply(max_supply);
        ctx.whitelist_admin_in_name_token();
        let token_hash = "token_hash";

        for i in 0..max_supply {
            // When minting a token
            ctx.token.mint(ctx.alice, i.into(), vec![]);
        }
        // When trying to mint a new token
        let result = ctx.token.try_mint(ctx.alice, max_supply.into(), vec![]);
        // Then it should fail with TokenSupplyDepleted error
        assert_eq!(
            result.err(),
            Some(NameTokenError::TokenSupplyDepleted.into())
        );
    }

    #[test]
    fn test_token_exists() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let token_hash = "token_hash";
        let token_id = generate_token_id(token_hash);
        // Token should not exist
        assert_eq!(ctx.token.token_exists(token_id), false);

        // Mint a token
        let token_owner = ctx.alice;
        mint_for(&mut ctx, token_owner, token_hash);
        // Then the token should exist
        assert_eq!(ctx.token.token_exists(token_id), true);

        // Burn the token
        whitelist_accounts(&mut ctx, vec![token_owner]);
        ctx.set_caller(token_owner);
        assert!(try_burn(&mut ctx, token_hash).is_ok());
        // Then the token should not exist anymore
        assert_eq!(ctx.token.token_exists(token_id), false);
    }

    #[test]
    fn owner_cannot_burn_until_whitelisted() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        // Given a token owned by alice
        let token_hash = "token_hash";
        let token_id = generate_token_id(token_hash);
        let token_owner = ctx.alice;
        mint_for(&mut ctx, token_owner, token_hash);

        // Then burning token by the owner fails
        ctx.set_caller(token_owner);
        let result = try_burn(&mut ctx, token_hash);
        assert_eq!(result.err(), Some(NameTokenError::NotWhitelisted.into()));

        // When whitelist the token owner
        whitelist_accounts(&mut ctx, vec![token_owner]);
        // Then the token owner should be able to burn the token
        ctx.set_caller(token_owner);
        let result = try_burn(&mut ctx, token_hash);
        assert!(result.is_ok());
    }

    #[test]
    fn anyone_whitelisted_can_burn() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (token_owner, anyone) = (ctx.alice, ctx.anyone);
        // Given a token owned by the token owner
        let token_hash = "token_hash";
        mint_for(&mut ctx, token_owner, token_hash);

        // Then anyone should not be able to burn the token
        ctx.set_caller(anyone);
        let result = try_burn(&mut ctx, token_hash);
        assert_eq!(result.err(), Some(NameTokenError::NotWhitelisted.into()));

        // When whitelist the anyone account
        whitelist_accounts(&mut ctx, vec![anyone]);
        // Then the anyone account should be able to burn the token
        ctx.set_caller(anyone);
        let result = try_burn(&mut ctx, token_hash);
        assert!(result.is_ok());
    }

    #[test]
    fn burning_burnt_token_should_fail() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        // Given a token owned by alice
        let alice = ctx.alice;
        let token_hash = "token hash";
        let token_id = generate_token_id(token_hash);
        mint_for(&mut ctx, alice, token_hash);
        // When whitelist the token owner
        whitelist_accounts(&mut ctx, vec![alice]);
        assert!(ctx.token.token_exists(token_id));
        ctx.set_caller(alice);
        // Then the token owner should be able to burn the token
        assert!(try_burn(&mut ctx, token_hash).is_ok());
        // Then burning the token again should fail
        assert!(try_burn(&mut ctx, token_hash).is_err());
    }

    #[test]
    fn burning_non_existent_token_should_fail() {
        let mut ctx = TestContext::install_raw();
        let alice = ctx.alice;
        // Given a non existent token
        let token_hash = "token hash";
        let token_id = generate_token_id(token_hash);
        assert!(!ctx.token.token_exists(token_id));

        // Then burning the token should fail
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        assert!(try_burn(&mut ctx, token_hash).is_err());
    }

    #[test]
    fn admin_transfer_of_multiple_tokens_should_work() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given two tokens owned by alice
        let token_hashes = vec!["token_hash1".to_string(), "token_hash2".to_string()];
        let token_ids = token_hashes
            .iter()
            .map(|token_hash| generate_token_id(token_hash))
            .collect::<Vec<_>>();
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When admin transfer the tokens to bob
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        ctx.token.admin_transfer(bob, token_ids.clone());
        // Then bob should be the owner of the tokens
        assert!(ctx.token.try_assert_is_owner(token_ids[0], bob).is_ok());
        assert!(ctx.token.try_assert_is_owner(token_ids[1], bob).is_ok());
    }

    #[test]
    fn admin_transfer_of_multiple_tokens_should_not_work_for_non_whitelisted_account() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given two tokens owned by alice
        let token_hashes = vec!["token_hash1".to_string(), "token_hash2".to_string()];
        let token_ids = token_hashes
            .iter()
            .map(|token_hash| generate_token_id(token_hash))
            .collect::<Vec<_>>();
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When non whitelisted account tries to transfer the tokens
        ctx.set_caller(alice);
        assert_eq!(
            ctx.token.try_admin_transfer(bob, token_ids.clone()),
            Err(NameTokenError::NotWhitelisted.into())
        );

        // Then bob should not be the owner of the tokens
        assert!(ctx.token.try_assert_is_owner(token_ids[0], bob).is_err());
        assert!(ctx.token.try_assert_is_owner(token_ids[1], bob).is_err());
    }

    #[test]
    fn admin_transfer_of_non_existent_token_should_fail() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given two tokens owned by alice
        let token_hashes = vec![
            "token_hash1".to_string(),
            "token_hash2".to_string(),
            "token_hash3".to_string(),
        ];
        let token_ids = token_hashes
            .iter()
            .map(|token_hash| generate_token_id(token_hash))
            .collect::<Vec<_>>();
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When admin transfer the tokens to bob
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        // Then the transfer fails
        assert!(ctx
            .token
            .try_admin_transfer(bob, token_ids.clone())
            .is_err());
        // Then the existing tokens should not be transferred
        assert!(ctx.token.try_assert_is_owner(token_ids[0], bob).is_err());
        assert!(ctx.token.try_assert_is_owner(token_ids[1], bob).is_err());
    }

    #[test]
    fn only_owner_can_set_resolver() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given a token owned by alice
        let token_hash = "token_hash";
        let token_id = generate_token_id(token_hash);
        mint_for(&mut ctx, alice, token_hash);
        // Then the token has no resolver
        assert_eq!(ctx.token.resolver(token_id), None);

        // When bob tries to set the resolver
        let resolver = bob;
        ctx.set_caller(bob);
        // Then the operation should fail
        assert_eq!(
            ctx.token.try_set_resolver(token_id, resolver),
            Err(NameTokenError::InvalidTokenOwner.into())
        );
        // When alice sets the resolver
        ctx.set_caller(alice);
        ctx.token.set_resolver(token_id, resolver);
        // Then the resolver should be set
        assert_eq!(ctx.token.resolver(token_id), Some(resolver));
    }

    #[test]
    fn test_is_token_valid() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // Given a token with expiration time in furure
        let token_hash = "token_hash";
        let token_id = generate_token_id(token_hash);
        let expiration = INIT_TIME + 100;
        ctx.set_caller(ctx.admin);
        let token_meta_data = NameTokenMetadata::with_no_resolver(token_hash, expiration);
        ctx.token.mint(alice, token_id, token_meta_data.to_vec());
        // Then the token should be valid
        assert!(ctx.token.is_token_valid(token_id));
        // When the expiration time is passed
        ctx.advance_block_time(expiration + 1);
        // Then the token should not be valid
        assert!(!ctx.token.is_token_valid(token_id));
    }

    #[test]
    fn burnt_token_is_invalid() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // Given a token with expiration time in furure
        let name = "token hash";
        let token_id = generate_token_id(name);
        let expiration = INIT_TIME + 100;
        ctx.set_caller(ctx.admin);
        let token_meta_data = NameTokenMetadata::with_no_resolver(name, expiration);
        ctx.token.mint(alice, token_id, token_meta_data.to_vec());
        // Then the token should be valid
        assert!(ctx.token.is_token_valid(token_id));

        // When the token is burnt
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        assert!(try_burn(&mut ctx, name).is_ok());
        // Then the token should not be valid
        assert!(!ctx.token.is_token_valid(token_id));
    }

    #[test]
    fn only_whitelisted_user_can_set_default_resolver() {
        let mut ctx = TestContext::install_raw();

        let resolver =
            Address::new("hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a")
                .unwrap();
        assert!(ctx.token.try_set_default_resolver(resolver).is_err());

        ctx.whitelist_admin_in_name_token();
        assert!(ctx.token.try_set_default_resolver(resolver).is_ok());

        assert_eq!(ctx.token.get_default_resolver(), resolver);
    }

    #[test]
    fn transfer_from_operator_resets_resolver() {
        let mut ctx = TestContext::install_and_setup();
        let (alice, bob, anyone) = (ctx.alice, ctx.bob, ctx.anyone);
        let token_label = "token-label";
        let full_domain = format!("{}.cspr", token_label);

        // Given Alice has a token.
        mint_for(&mut ctx, alice, token_label);

        // Alice sets the resolver.
        ctx.set_caller(alice);
        ctx.default_resolver
            .set_resolution(full_domain.clone(), Some(anyone));

        // Then the resolver points at some address.
        assert_eq!(
            ctx.default_resolver.resolve(full_domain.clone()),
            Some(anyone)
        );

        // Given Alice sets Bob as an operator.
        ctx.token.approve_for_all(bob);

        // Then Bob is an operator for Alice.
        assert!(ctx.token.is_approved_for_all(alice, bob));

        // When Bob transfers the token to himself.
        let token_id = generate_token_id(token_label);
        ctx.set_caller(bob);
        ctx.token.transfer_from(alice, bob, token_id);
        assert!(ctx.token.try_assert_is_owner(token_id, bob).is_ok());

        // Then the resolver is reset.
        assert_eq!(
            ctx.default_resolver.resolve(full_domain),
            None,
            "Resolver should be reset after transfer from operator"
        );
    }

    #[test]
    fn test_revoke_whitelist() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // Given Alice is whitelisted
        whitelist_accounts(&mut ctx, vec![alice]);

        // When admin revokes Alice's whitelist
        ctx.set_caller(ctx.admin);
        let result = ctx.token.try_revoke_whitelist(alice);
        // Then it should succeed
        assert!(result.is_ok());

        // When admin tries to revoke Alice's whitelist again
        let result = ctx.token.try_revoke_whitelist(alice);
        // Then it should fail with NotWhitelisted error
        assert_eq!(result.err(), Some(NameTokenError::NotWhitelisted.into()));
    }

    #[test]
    fn test_whitelist() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // When admin tries to whitelist Alice
        ctx.set_caller(ctx.admin);
        let result = ctx.token.try_whitelist(alice);
        // Then it should succeed
        assert!(result.is_ok());

        // When admin tries to whitelist Alice again
        let result = ctx.token.try_whitelist(alice);
        // Then it should fail with WhitelistedAlready error
        assert_eq!(
            result.err(),
            Some(NameTokenError::WhitelistedAlready.into())
        );
    }

    fn mint_for(ctx: &mut TestContext, owner: Address, name: &str) -> U256 {
        ctx.set_caller(ctx.admin);
        let token_metadata =
            NameTokenMetadata::with_no_resolver(name, INIT_TIME + TOKEN_EXPIRATION);
        let token_id = generate_token_id(name);
        ctx.token.mint(owner, token_id, token_metadata.to_vec());
        token_id
    }

    fn whitelist_accounts(ctx: &mut TestContext, accounts: Vec<Address>) {
        ctx.set_caller(ctx.admin);
        for account in accounts {
            ctx.token.whitelist(account);
        }
    }

    fn try_burn(ctx: &mut TestContext, token_hash: &str) -> OdraResult<()> {
        let token_id = generate_token_id(token_hash);
        ctx.token.try_burn(token_id)
    }
}
