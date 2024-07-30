use crate::data_structures::NameTokenMetadata;
use odra::args::Maybe;
use odra::module::Revertible;
use odra::{prelude::*, UnwrapOrRevert};
use odra::{Address, SubModule};
use odra_modules::cep78::modalities::{
    BurnMode, EventsMode, MetadataMutability, MintingMode, NFTHolderMode, NFTIdentifierMode,
    NFTKind, NFTMetadataKind, OwnershipMode, WhitelistMode,
};
use odra_modules::cep78::token::Cep78;

#[odra::module]
pub struct NameToken {
    token: SubModule<Cep78>,
}

#[odra::module]
impl NameToken {
    delegate! {
        to self.token {
            fn get_collection_name(&self) -> String;
            fn get_collection_symbol(&self) -> String;
            fn set_variables(
                &mut self,
                allow_minting: Maybe<bool>,
                acl_whitelist: Maybe<Vec<Address>>,
                operator_burn_mode: Maybe<bool>
            );
            fn mint(
                &mut self,
                token_owner: Address,
                token_meta_data: String,
                token_hash: Maybe<String>
            );
            // fn burn(&mut self, token_id: Maybe<u64>, token_hash: Maybe<String>);
            fn transfer(
                &mut self,
                token_id: Maybe<u64>,
                token_hash: Maybe<String>,
                source_key: Address,
                target_key: Address
            );
            fn approve(&mut self, spender: Address, token_id: Maybe<u64>, token_hash: Maybe<String>);
            fn revoke(&mut self, token_id: Maybe<u64>, token_hash: Maybe<String>);
            fn set_approval_for_all(&mut self, approve_all: bool, operator: Address);
            fn is_approved_for_all(&mut self, token_owner: Address, operator: Address) -> bool;
            fn owner_of(&self, token_id: Maybe<u64>, token_hash: Maybe<String>) -> Address;
            fn get_approved(
                &mut self,
                token_id: Maybe<u64>,
                token_hash: Maybe<String>
            ) -> Option<Address>;
            fn metadata(&self, token_id: Maybe<u64>, token_hash: Maybe<String>) -> String;
            // fn set_token_metadata(
            //     &mut self,
            //     token_id: Maybe<u64>,
            //     token_hash: Maybe<String>,
            //     token_meta_data: String
            // );
            fn balance_of(&mut self, token_owner: Address) -> u64;
            fn register_owner(&mut self, token_owner: Maybe<Address>) -> String;
        }
    }

    pub fn init(&mut self, name: String, symbol: String) {
        // Setup CEP78 token.
        let max_total_supply = 1_000_000u64;
        let ownership_mode = OwnershipMode::Transferable;
        let nft_kind = NFTKind::Digital;
        let identifier_mode = NFTIdentifierMode::Hash;
        let nft_metadata_kind = NFTMetadataKind::Raw;
        let metadata_mutability = MetadataMutability::Mutable;
        let receipt_name = String::new();
        let allow_minting = Maybe::Some(true);
        let minting_mode = Maybe::Some(MintingMode::Acl);
        let holder_mode = Maybe::Some(NFTHolderMode::Mixed);
        let whitelist_mode = Maybe::Some(WhitelistMode::Unlocked);
        let acl_white_list = Maybe::None;
        let json_schema = Maybe::None;
        let burn_mode = Maybe::Some(BurnMode::Burnable);
        let operator_burn_mode = Maybe::None; // ?
        let owner_reverse_lookup_mode = Maybe::None; // ?
        let events_mode = Maybe::Some(EventsMode::CES);
        let transfer_filter_contract_contract = Maybe::None; // ?
        let additional_required_metadata = Maybe::None; // ?
        let optional_metadata = Maybe::Some(vec![]); // ?
        self.token.init(
            name,
            symbol,
            max_total_supply,
            ownership_mode,
            nft_kind,
            identifier_mode,
            nft_metadata_kind,
            metadata_mutability,
            receipt_name,
            allow_minting,
            minting_mode,
            holder_mode,
            whitelist_mode,
            acl_white_list,
            json_schema,
            burn_mode,
            operator_burn_mode,
            owner_reverse_lookup_mode,
            events_mode,
            transfer_filter_contract_contract,
            additional_required_metadata,
            optional_metadata,
        );
    }

    pub fn token_exists(&self, token_hash: &String) -> bool {
        self.token.token_exists_by_hash(token_hash)
    }

    pub fn burn(&mut self, token_id: Maybe<u64>, token_hash: Maybe<String>) {
        let caller = self.env().caller();
        if !self.token.is_whitelisted(&caller) {
            self.revert(NameTokenError::NotWhitelisted);
        }

        let token_identifier = self.token.token_identifier(token_id, token_hash);
        let token_id = token_identifier.to_string();
        self.token.burn_token_unchecked(token_id, caller);
    }

    pub fn admin_transfer(&mut self, reciepient: Address, token_hashes: Vec<String>) {
        let spender = self.env().caller();
        if !self.token.is_whitelisted(&spender) {
            self.revert(NameTokenError::NotWhitelisted);
        }
        for token_hash in token_hashes {
            let owner = self.token.owner_of_by_id(&token_hash);
            self.token
                .transfer_unchecked(token_hash, owner, Some(spender), reciepient);
        }
    }

    pub fn set_token_metadata(
        &mut self,
        token_id: Maybe<u64>,
        token_hash: Maybe<String>,
        token_meta_data: String,
    ) {
        let caller = self.env().caller();
        if !self.token.is_whitelisted(&caller) {
            self.revert(NameTokenError::NotWhitelisted);
        }
        let token_id = self
            .token
            .token_identifier(token_id, token_hash)
            .to_string();
        self.token
            .set_token_metadata_unchecked(&token_id, token_meta_data);
    }

    pub fn metadata_by_hash(&self, token_hash: &String) -> String {
        self.metadata(Maybe::None, Maybe::Some(token_hash.clone()))
    }

    pub fn resolver(&self, token_id: String) -> Option<Address> {
        let metadata: NameTokenMetadata = self.wrapped_metadata(&token_id);
        metadata.resolver().unwrap_or_revert(self)
    }

    pub fn set_resolver(&mut self, token_id: String, resolver: Address) {
        if self.token.owner_of_by_id(&token_id) != self.env().caller() {
            self.revert(NameTokenError::InvalidTokenOwner);
        }
        let mut metadata: NameTokenMetadata = self.wrapped_metadata(&token_id);
        metadata.set_resolver(resolver);
        self.token
            .set_token_metadata_unchecked(&token_id, metadata.json());
    }

    pub fn assert_is_owner(&self, token_id: &String, address: Address) {
        let owner = self.token.owner_of_by_id(token_id);
        if owner != address {
            self.revert(NameTokenError::InvalidTokenOwner);
        }
    }

    pub fn is_token_valid(&self, token_hash: &String) -> bool {
        if !self.token.token_exists_by_hash(token_hash) {
            return false;
        }

        let metadata: NameTokenMetadata = self.wrapped_metadata(token_hash);
        if metadata.expiration().unwrap_or_revert(self) < self.env().get_block_time() {
            return false;
        }
        true
    }
}

impl NameToken {
    #[inline]
    pub fn wrapped_metadata(&self, token_hash: &String) -> NameTokenMetadata {
        self.metadata_by_hash(token_hash)
            .try_into()
            .unwrap_or_revert(self)
    }
}

#[odra::odra_error]
pub enum NameTokenError {
    NotWhitelisted = 1301,
    InvalidTokenOwner = 1302,
}

#[cfg(test)]
mod tests {
    use odra::OdraResult;

    use super::*;
    use crate::test_context::{TestContext, INIT_TIME};

    #[test]
    fn test_token_exists() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let token_hash = "token_hash";
        // Token should not exist
        assert_eq!(ctx.token.token_exists(&token_hash.to_string()), false);

        // Mint a token
        let token_owner = ctx.alice;
        mint_for(&mut ctx, token_owner, token_hash);
        // Then the token should exist
        assert_eq!(ctx.token.token_exists(&token_hash.to_string()), true);

        // Burn the token
        whitelist_accounts(&mut ctx, vec![token_owner]);
        ctx.set_caller(token_owner);
        assert!(try_burn(&mut ctx, token_hash).is_ok());
        // Then the token should not exist anymore
        assert_eq!(ctx.token.token_exists(&token_hash.to_string()), false);
    }

    #[test]
    fn owner_cannot_burn_until_whitelisted() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        // Given a token owned by alice
        let token_hash = "token_hash";
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
        mint_for(&mut ctx, alice, token_hash);
        // When whitelist the token owner
        whitelist_accounts(&mut ctx, vec![alice]);
        assert!(ctx.token.token_exists(&token_hash.to_string()));
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
        assert!(!ctx.token.token_exists(&token_hash.to_string()));

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
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When admin transfer the tokens to bob
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        ctx.token.admin_transfer(bob, token_hashes.clone());
        // Then bob should be the owner of the tokens
        assert!(ctx.token.try_assert_is_owner(&token_hashes[0], bob).is_ok());
        assert!(ctx.token.try_assert_is_owner(&token_hashes[1], bob).is_ok());
    }

    #[test]
    fn admin_transfer_of_multiple_tokens_should_not_work_for_non_whitelisted_account() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given two tokens owned by alice
        let token_hashes = vec!["token_hash1".to_string(), "token_hash2".to_string()];
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When non whitelisted account tries to transfer the tokens
        ctx.set_caller(alice);
        assert_eq!(
            ctx.token.try_admin_transfer(bob, token_hashes.clone()),
            Err(NameTokenError::NotWhitelisted.into())
        );

        // Then bob should not be the owner of the tokens
        assert!(ctx
            .token
            .try_assert_is_owner(&token_hashes[0], bob)
            .is_err());
        assert!(ctx
            .token
            .try_assert_is_owner(&token_hashes[1], bob)
            .is_err());
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
        mint_for(&mut ctx, alice, &token_hashes[0]);
        mint_for(&mut ctx, alice, &token_hashes[1]);

        // When admin transfer the tokens to bob
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        // Then the transfer fails
        assert!(ctx
            .token
            .try_admin_transfer(bob, token_hashes.clone())
            .is_err());
        // Then the existing tokens should not be transferred
        assert!(ctx
            .token
            .try_assert_is_owner(&token_hashes[0], bob)
            .is_err());
        assert!(ctx
            .token
            .try_assert_is_owner(&token_hashes[1], bob)
            .is_err());
    }

    #[test]
    fn only_owner_can_set_resolver() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let (alice, bob) = (ctx.alice, ctx.bob);
        // Given a token owned by alice
        let token_hash = "token_hash";
        mint_for(&mut ctx, alice, token_hash);
        // Then the token has no resolver
        assert_eq!(ctx.token.resolver(token_hash.to_owned()), None);

        // When bob tries to set the resolver
        let resolver = bob;
        ctx.set_caller(bob);
        // Then the operation should fail
        assert_eq!(
            ctx.token.try_set_resolver(token_hash.to_owned(), resolver),
            Err(NameTokenError::InvalidTokenOwner.into())
        );
        // When alice sets the resolver
        ctx.set_caller(alice);
        ctx.token.set_resolver(token_hash.to_owned(), resolver);
        // Then the resolver should be set
        assert_eq!(ctx.token.resolver(token_hash.to_owned()), Some(resolver));
    }

    #[test]
    fn test_is_token_valid() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // Given a token with expiration time in furure
        let name = "token hash";
        let expiration = INIT_TIME + 100;
        ctx.set_caller(ctx.admin);
        let token_meta_data = NameTokenMetadata::with_no_resolver(name, expiration);
        ctx.token
            .mint(alice, token_meta_data.json(), Maybe::Some(name.to_owned()));
        // Then the token should be valid
        assert!(ctx.token.is_token_valid(&name.to_string()));
        // When the expiration time is passed
        ctx.advance_block_time(expiration + 1);
        // Then the token should not be valid
        assert!(!ctx.token.is_token_valid(&name.to_string()));
    }

    #[test]
    fn burnt_token_is_invalid() {
        let mut ctx = TestContext::install_raw();
        ctx.whitelist_admin_in_name_token();
        let alice = ctx.alice;

        // Given a token with expiration time in furure
        let name = "token hash";
        let expiration = INIT_TIME + 100;
        ctx.set_caller(ctx.admin);
        let token_meta_data = NameTokenMetadata::with_no_resolver(name, expiration);
        ctx.token
            .mint(alice, token_meta_data.json(), Maybe::Some(name.to_owned()));
        // Then the token should be valid
        assert!(ctx.token.is_token_valid(&name.to_string()));

        // When the token is burnt
        whitelist_accounts(&mut ctx, vec![alice]);
        ctx.set_caller(alice);
        assert!(try_burn(&mut ctx, name).is_ok());
        // Then the token should not be valid
        assert!(!ctx.token.is_token_valid(&name.to_string()));
    }

    fn mint_for(ctx: &mut TestContext, owner: Address, name: &str) {
        ctx.set_caller(ctx.admin);
        let token_meta_data = NameTokenMetadata::with_no_resolver(name, 0);
        ctx.token
            .mint(owner, token_meta_data.json(), Maybe::Some(name.to_owned()));
    }

    fn whitelist_accounts(ctx: &mut TestContext, accounts: Vec<Address>) {
        ctx.set_caller(ctx.admin);
        ctx.token
            .set_variables(Maybe::None, Maybe::Some(accounts), Maybe::None);
    }

    fn try_burn(ctx: &mut TestContext, token_hash: &str) -> OdraResult<()> {
        ctx.token
            .try_burn(Maybe::None, Maybe::Some(token_hash.to_owned()))
    }
}
