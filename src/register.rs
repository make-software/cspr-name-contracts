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

type TokenHash = odra::prelude::String;

// TODO: Match roles from the Solidity code.
pub const MINTER_ROLE: Role = [1; 32];
pub const OPERATOR_ROLE: Role = [2; 32];

#[odra::odra_error]
pub enum RegisterError {
    EmptyTLD = 1001,
    TLDNotSupported = 1002,
    PastExpirationDate = 1003,
    EmptyLabel = 1004,
    SLDDoesNotExist = 1005,
}

#[odra::event]
pub struct SLDMinted {
    token_hash: TokenHash,
    to: Address,
    label: String,
    expiration: u64,
}

#[odra::module]
pub struct Register {
    access_control: SubModule<AccessControl>,
    token: SubModule<Cep78>,
}

#[odra::module]
impl Register {
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

        self.mint_single(to, &token_hash, expiration);
    }

    pub fn renew(&mut self, token_hash: TokenHash, expiration: u64) {
        self.assert_minter_role();
        self.require_future_expiration_date(expiration);
        self.require_token_minted(&token_hash);
        self.set_expiration(&token_hash, expiration);
    }

    pub fn admin_burn(&mut self, token_hash: TokenHash) {
        self.assert_operator_role();
        self.burn_single(token_hash, self.env().caller());
    }

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
            ) -> (String, Address);
            fn approve(&mut self, spender: Address, token_id: Maybe<u64>, token_hash: Maybe<String>);
            fn set_approval_for_all(&mut self, approve_all: bool, operator: Address);
            fn balance_of(&mut self, token_owner: Address) -> u64;
            fn owner_of(&self, token_id: Maybe<u64>, token_hash: Maybe<String>) -> Address;
            fn get_approved(
                &mut self,
                token_id: Maybe<u64>,
                token_hash: Maybe<String>
            ) -> Option<Address>;
        }

        to self.access_control {
            fn grant_role(&mut self, role: &Role, address: &Address);
            // TODO: Decide on role management public functions.
        }
    }

    pub fn burn(&self) {}
    pub fn set_token_metadata(&mut self) {}
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

    fn mint_single(&mut self, to: Address, token_hash: &TokenHash, expiration: u64) {
        let metadata = expiration.to_string();
        let token_hash = Maybe::Some(String::from(token_hash));
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
        let metadata = self
            .token
            .metadata(Maybe::None, Maybe::Some(String::from(token_hash)));
        u64::from_str(&metadata).ok().unwrap_or_revert(&self.env())
    }

    fn set_expiration(&mut self, token_hash: &TokenHash, expiration: u64) {
        self.token
            .set_token_metadata_unchecked(token_hash, expiration.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv, HostRef};
    use odra_modules::cep78::events::{Burn, MetadataUpdated, Mint};

    const NFT_NAME: &'static str = "D3 Tokens";
    const NFT_SYMBOL: &'static str = "D3";
    const TEST_LABEL: &'static str = "test-label";
    const TEST_LABEL_BLAKE2B: &'static str =
        "44b7dfe6596e4668313215e4f12ee9650d911a59087944b077d5c087212b89b1";
    const BASE_URI: &'static str = "https://storage.test/";
    const ONE_DAY_SECONDS: u64 = 60 * 60 * 24;

    struct RegisterTestContext {
        pub register: RegisterHostRef,
        pub env: HostEnv,
        pub owner: Address,
        pub operator: Address,
        pub minter: Address,
        pub user: Address,
    }

    impl RegisterTestContext {
        fn new() -> Self {
            let env = odra_test::env();
            let owner = env.get_account(0);
            let operator = env.get_account(1);
            let minter = env.get_account(2);
            let user = env.get_account(3);
            let register = RegisterHostRef::deploy(
                &env,
                RegisterInitArgs {
                    name: String::from(NFT_NAME),
                    symbol: String::from(NFT_SYMBOL),
                },
            );
            Self {
                register,
                env,
                owner,
                operator,
                minter,
                user,
            }
        }

        pub fn add_minter_role(&mut self) {
            self.register.grant_role(&MINTER_ROLE, &self.minter);
        }

        pub fn add_operator_role(&mut self) {
            self.register.grant_role(&OPERATOR_ROLE, &self.operator);
        }
    }

    mod initialize {

        use super::*;

        #[test]
        fn should_set_correct_name_and_symbol() {
            let ctx = RegisterTestContext::new();
            assert_eq!(ctx.register.get_collection_name(), NFT_NAME);
            assert_eq!(ctx.register.get_collection_symbol(), NFT_SYMBOL);
        }

        #[test]
        fn should_emit_correct_tld_events() {
            // let ctx = RegisterTestContext::new();
            // // TODO: Assert access_control events.
            // assert_eq!(ctx.env.events_count(ctx.register.address()), 5);
        }
    }

    mod supports_interface {
        // #[test]
        // fn should_support_erc721() {
        // let ctx = RegisterTestContext::new();
        // assert!(ctx.register.supports_interface("0x80ac58cd"));
        // }
    }

    mod mint {
        use odra_modules::cep78::events::Mint;

        use super::*;

        #[test]
        fn should_mint_sld_nft() {
            let mut ctx = RegisterTestContext::new();
            ctx.add_minter_role();

            ctx.env.set_caller(ctx.minter);
            ctx.register
                .mint(ctx.user, String::from(TEST_LABEL), ONE_DAY_SECONDS);

            let event: Mint = ctx.register.get_event(-1).unwrap();
            let expected = Mint::new(
                ctx.user,
                String::from(TEST_LABEL_BLAKE2B),
                ONE_DAY_SECONDS.to_string(),
            );
            assert_eq!(event, expected);
        }
    }

    #[test]
    fn should_mint_renew_and_burn() {
        let mut ctx = RegisterTestContext::new();
        ctx.add_minter_role();
        ctx.add_operator_role();
        let token_id = String::from(TEST_LABEL_BLAKE2B);

        ctx.env.set_caller(ctx.minter);
        ctx.register
            .mint(ctx.user, String::from(TEST_LABEL), ONE_DAY_SECONDS);
        let event: Mint = ctx.register.get_event(-1).unwrap();
        let expected = Mint::new(ctx.user, token_id.clone(), ONE_DAY_SECONDS.to_string());
        assert_eq!(event, expected);

        ctx.register.renew(token_id.clone(), ONE_DAY_SECONDS * 2);
        let event: MetadataUpdated = ctx.register.get_event(-1).unwrap();
        let expected = MetadataUpdated::new(token_id.clone(), (ONE_DAY_SECONDS * 2).to_string());
        assert_eq!(event, expected);

        ctx.env.set_caller(ctx.operator);
        ctx.register.admin_burn(token_id.clone());
        let event: Burn = ctx.register.get_event(-1).unwrap();
        let expected = Burn::new(ctx.user, token_id, ctx.operator);
        assert_eq!(event, expected);
    }
}
