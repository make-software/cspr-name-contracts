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
        let minting_mode = Maybe::Some(MintingMode::Public);
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

    // TODO: Test this function.
    pub fn token_exists(&self, token_hash: &String) -> bool {
        self.token.token_exists_by_hash(token_hash)
    }

    // TODO: Test this function, make sure only admin can call it.
    pub fn burn(&mut self, token_id: Maybe<u64>, token_hash: Maybe<String>) {
        let caller = self.env().caller();
        if !self.token.is_whitelisted(&caller) {
            self.revert(NameTokenError::NotWhitelisted);
        }

        let token_identifier = self.token.token_identifier(token_id, token_hash);
        let token_id = token_identifier.to_string();
        self.token.burn_token_unchecked(token_id, caller);
    }

    // TODO: Test this function.
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

    // TODO: Test this function.
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

    pub fn metadata_by_hash(&self, token_hash: &String) -> NameTokenMetadata {
        self._metadata_by_hash(token_hash.to_string())
    }

    pub fn resolver(&self, token_id: String) -> Option<Address> {
        let metadata = self._metadata_by_hash(token_id);
        metadata.resolver
    }

    pub fn set_resolver(&mut self, token_id: String, resolver: Address) {
        if self.token.owner_of_by_id(&token_id) != self.env().caller() {
            self.revert(NameTokenError::InvalidTokenOwner);
        }
        let metadata = self._metadata_by_hash(token_id.clone());
        let new_metadata = NameTokenMetadata {
            resolver: Some(resolver),
            ..metadata
        };
        self.set_token_metadata(
            Maybe::None,
            Maybe::Some(token_id),
            new_metadata.to_json().unwrap_or_revert(self),
        );
    }
}

impl NameToken {
    #[inline]
    fn _metadata_by_hash(&self, token_hash: String) -> NameTokenMetadata {
        let metadata = self.metadata(Maybe::None, Maybe::Some(token_hash));
        NameTokenMetadata::from_json(&metadata).unwrap_or_revert(self)
    }
}

#[odra::odra_error]
pub enum NameTokenError {
    NotWhitelisted = 3001,
    InvalidTokenOwner = 3002,
}
