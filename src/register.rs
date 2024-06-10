use odra::args::Maybe;
use odra::prelude::*;
use odra::{Address, SubModule, UnwrapOrRevert};
use odra_modules::cep78::modalities::{
    BurnMode, EventsMode, MetadataMutability, MintingMode, NFTHolderMode, NFTIdentifierMode,
    NFTKind, NFTMetadataKind, OwnershipMode, WhitelistMode,
};
use odra_modules::{
    access::{AccessControl, Role, DEFAULT_ADMIN_ROLE},
    cep78::token::Cep78,
};
use serde::{Deserialize, Serialize};

type TokenHash = odra::prelude::String;

// TODO: Match roles from the Solidity code.
pub const MINTER_ROLE: Role = [1; 32];
pub const OPERATOR_ROLE: Role = [2; 32];

#[odra::odra_error]
#[derive(Debug)]
pub enum RegisterError {
    EmptyTLD = 1001,
    TLDNotSupported = 1002,
    PastExpirationDate = 1003,
    EmptyLabel = 1004,
    SLDDoesNotExist = 1005,
    SerializationError = 1006,
    DeserializationError = 1007,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SLDMetadata {
    pub label: String,
    pub expiration: u64,
}

impl SLDMetadata {
    pub fn new(label: String, expiration: u64) -> Self {
        Self {
            label,
            expiration,
        }
    }
    
    pub fn to_json(&self) -> Result<String, RegisterError> {
        serde_json_wasm::to_string(self).map_err(|_| RegisterError::SerializationError)
    }

    pub fn from_json(json: &str) -> Result<Self, RegisterError> {
        serde_json_wasm::from_str(json).map_err(|_| RegisterError::DeserializationError)
    }
}

#[odra::odra_type]
pub struct NameMintInfo {
    label: String,
    expiration_time: u64,
    owner: Address
}

impl NameMintInfo {
    pub fn new(label: &str, expiration_time: u64, owner: Address) -> Self {
        Self {
            label: String::from(label),
            expiration_time,
            owner
        }
    }
}

#[odra::module]
pub struct Register {
    access_control: SubModule<AccessControl>,
    token: SubModule<Cep78>,
}

#[odra::module]
impl Register {
    delegate! {
        to self.token {
            fn get_collection_name(&self) -> String;
            fn get_collection_symbol(&self) -> String;
            fn transfer(
                &mut self,
                token_id: Maybe<u64>,
                token_hash: Maybe<String>,
                source_key: Address,
                target_key: Address
            );
            fn approve(&mut self, spender: Address, token_id: Maybe<u64>, token_hash: Maybe<String>);
            fn set_approval_for_all(&mut self, approve_all: bool, operator: Address);
            fn balance_of(&mut self, token_owner: Address) -> u64;
            fn owner_of(&self, token_id: Maybe<u64>, token_hash: Maybe<String>) -> Address;
            fn get_approved(
                &mut self,
                token_id: Maybe<u64>,
                token_hash: Maybe<String>
            ) -> Option<Address>;
            fn metadata(&self, token_id: Maybe<u64>, token_hash: Maybe<String>) -> String;
        }

        to self.access_control {
            fn grant_role(&mut self, role: &Role, address: &Address);
            // TODO: Decide on role management public functions.
        }
    }

    pub fn init(&mut self, name: String, symbol: String) {
        let caller = self.env().caller();

        // Setup access control.
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &caller);
        self.access_control
            .set_admin_role(&MINTER_ROLE, &DEFAULT_ADMIN_ROLE);
        self.access_control
            .set_admin_role(&OPERATOR_ROLE, &DEFAULT_ADMIN_ROLE);

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

    // TODO: merge check and get.
    pub fn mint(&mut self, to: Address, label: String, expiration: u64) {
        self.assert_minter_role();
        self.require_future_expiration_date(expiration);

        if label.is_empty() {
            self.env().revert(RegisterError::EmptyLabel);
        }

        let token_hash = self.compute_namehash(&label);

        if self.token_exists(&token_hash) && self.is_token_expired(&token_hash) {
            self.burn_single(token_hash.clone(), self.env().caller());
        }

        self.mint_single(to, label, &token_hash, expiration);
    }

    pub fn bulk_mint(&mut self, names: Vec<NameMintInfo>) {
        self.assert_minter_role();
        for name in names {
            self.mint(name.owner, name.label, name.expiration_time);
        }
    }

    pub fn renew(&mut self, token_hash: TokenHash, expiration: u64) {
        self.assert_minter_role();
        self.require_future_expiration_date(expiration);
        self.require_token_minted(&token_hash);
        self.set_expiration(&token_hash, expiration);
    }

    pub fn burn(&self) {}
    pub fn set_token_metadata(&mut self) {}
    
    // TODO: Read metadata.
    // pub fn metadata(&self) {}
}

impl Register {
    // TODO: Make sure this implementation is sufficient.
    fn compute_namehash(&self, label: &str) -> TokenHash {
        let hash = self.env().hash(label);
        hex::encode(hash)
    }

    fn require_future_expiration_date(&self, expiration: u64) {
        if expiration < self.env().get_block_time() {
            self.env().revert(RegisterError::PastExpirationDate);
        }
    }

    fn require_token_minted(&self, token_hash: &TokenHash) {
        if !self.token_exists(token_hash) {
            self.env().revert(RegisterError::SLDDoesNotExist);
        }
    }

    fn token_exists(&self, token_hash: &TokenHash) -> bool {
        self.token.token_exists_by_hash(token_hash)
    }

    fn is_token_expired(&self, token_hash: &TokenHash) -> bool {
        self.expiration(token_hash) < self.env().get_block_time()
    }

    fn burn_single(&mut self, token_hash: TokenHash, burner: Address) {
        self.token.burn_token_unchecked(token_hash, burner);
    }

    fn mint_single(&mut self, to: Address, label: String, token_hash: &TokenHash, expiration: u64) {
        let metadata = SLDMetadata::new(label, expiration);
        let metadata = metadata.to_json().unwrap_or_revert(&self.env());
        let token_hash = Maybe::Some(token_hash.clone());
        self.token.mint(to, metadata, token_hash);
    }

    fn assert_minter_role(&self) {
        self.access_control
            .check_role(&MINTER_ROLE, &self.env().caller());
    }

    fn assert_operator_role(&self) {
        self.access_control
            .check_role(&OPERATOR_ROLE, &self.env().caller());
    }

    fn expiration(&self, token_hash: &TokenHash) -> u64 {
        let metadata = self.get_metadata(token_hash);
        metadata.expiration
    }

    fn set_expiration(&mut self, token_hash: &TokenHash, expiration: u64) {
        let mut metadata = self.get_metadata(token_hash);
        metadata.expiration = expiration;
        let metadata = metadata.to_json().unwrap_or_revert(&self.env());
        self.token
            .set_token_metadata_unchecked(token_hash, metadata);
    }

    fn get_metadata(&self, token_hash: &TokenHash) -> SLDMetadata {
        let metadata = self
            .token
            .metadata(Maybe::None, Maybe::Some(String::from(token_hash)));
        SLDMetadata::from_json(&metadata).unwrap_or_revert(&self.env())
    }
}
