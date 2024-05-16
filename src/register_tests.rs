use blake2::{digest::VariableOutput, Blake2bVar};
use crate::register::{RegisterHostRef, RegisterInitArgs, SLDMetadata, MINTER_ROLE, OPERATOR_ROLE};
use odra::{args::Maybe, host::{Deployer, HostEnv, HostRef}, Address};
use odra_modules::cep78::events::{Burn, MetadataUpdated, Mint};
use odra_modules::access::errors::Error as AccessControlError;
use std::io::Write;

const NFT_NAME: &'static str = "D3 Tokens";
const NFT_SYMBOL: &'static str = "D3";
const TEST_LABEL: &'static str = "test-label";
const TEST_LABEL_BLAKE2B: &'static str =
    "44b7dfe6596e4668313215e4f12ee9650d911a59087944b077d5c087212b89b1";
const BASE_URI: &'static str = "https://storage.test/";
const ONE_DAY: u64 = 60 * 60 * 24 * 100;

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

    pub fn mint_token(&mut self) -> (String, u64) {
        self.add_minter_role();
        self.env.set_caller(self.minter);
        let token_id = blake2b(TEST_LABEL);
        let expiration_time = self.get_token_expiration();
        
        self.register
            .mint(self.user, String::from(TEST_LABEL), expiration_time);
        // self.env.advance_block_time(1);
        (token_id, expiration_time)
    }

    pub fn assert_token_minted(&self, token_id: &str, expiration_time: u64, label: &str) {
        self.env.emitted_event(self.register.address(), &Mint::new(
            self.user,
            String::from(token_id),
            SLDMetadata::new(String::from(label), expiration_time).to_json().unwrap(),
        ));

        // Token URI
        // TODO: Test non existing tokens.

        // Make sure token owner is correct
        let owner = self.register.owner_of(Maybe::None, Maybe::Some(String::from(token_id)));
        assert_eq!(owner, self.user);

        // Verify correct expiration date is set
        let metadata = self.register.metadata(Maybe::None, Maybe::Some(String::from(token_id)));
        let metadata = SLDMetadata::from_json(&metadata).unwrap();
        assert_eq!(metadata.expiration, expiration_time);
        assert_eq!(metadata.label, label);
    }

    pub fn get_token_expiration(&self) -> u64 {
        let now = self.env.block_time();
        now + ONE_DAY
    }

}

fn blake2b<T: AsRef<[u8]>>(data: T) -> String {
    let mut result = [0u8; 32];
    let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
    let _ = hasher.write(data.as_ref());
    hasher
        .finalize_variable(&mut result)
        .expect("should copy hash to the result array");
    hex::encode(result)
}

mod initialize {

    use odra_modules::access::{events::{RoleAdminChanged, RoleGranted}, DEFAULT_ADMIN_ROLE};

    use super::*;

    #[test]
    fn should_set_correct_name_and_symbol() {
        let ctx = RegisterTestContext::new();
        // assert_eq!(ctx.register.get_collection_name(), NFT_NAME);
        // assert_eq!(ctx.register.get_collection_symbol(), NFT_SYMBOL);
    }

    #[test]
    fn should_emit_correct_events() {
        let ctx = RegisterTestContext::new();
        
        // First event.
        ctx.env.emitted_event(ctx.register.address(), &RoleGranted {
            role: DEFAULT_ADMIN_ROLE,
            address: ctx.owner,
            sender: ctx.owner,
        });

        // Second event.
        ctx.env.emitted_event(ctx.register.address(), &RoleAdminChanged {
            role: MINTER_ROLE,
            previous_admin_role: DEFAULT_ADMIN_ROLE,
            new_admin_role: DEFAULT_ADMIN_ROLE,
        });

        // Third event.
        ctx.env.emitted_event(ctx.register.address(), &RoleAdminChanged {
            role: OPERATOR_ROLE,
            previous_admin_role: DEFAULT_ADMIN_ROLE,
            new_admin_role: DEFAULT_ADMIN_ROLE,
        });
        

        assert_eq!(ctx.env.events_count(ctx.register.address()), 3);
    }
}

mod supports_interface {
    #[test]
    #[ignore]
    fn should_support_erc721() {}
}

mod mint {
    use odra_modules::cep78::error::CEP78Error;

    use crate::register::RegisterError;

    use super::*;

    #[test]
    fn should_mint_sld_nft() {
        let mut ctx = RegisterTestContext::new();
        
        let (token_id, expiration_time) = ctx.mint_token();
        ctx.assert_token_minted(&token_id, expiration_time, TEST_LABEL);
    }

    #[test]
    fn should_fail_when_called_by_a_non_minter() {
        let mut ctx = RegisterTestContext::new();
        ctx.env.set_caller(ctx.user);
        let result = ctx.register
            .try_mint(ctx.user, String::from(TEST_LABEL), ONE_DAY);
        assert_eq!(result.unwrap_err(), AccessControlError::MissingRole.into());
    }

    #[test]
    #[ignore]
    fn should_fail_when_tld_is_not_supported() {}

    #[test]
    fn should_fail_when_label_is_empty() {
        let mut ctx = RegisterTestContext::new();
        ctx.add_minter_role();
        ctx.env.set_caller(ctx.minter);
        let result = ctx.register
            .try_mint(ctx.user, String::from(""), ONE_DAY);
        assert_eq!(result.unwrap_err(), RegisterError::EmptyLabel.into());
    }

    #[test]
    fn should_fail_when_expiration_date_is_in_the_past() {
        let mut ctx = RegisterTestContext::new();
        ctx.add_minter_role();
        ctx.env.advance_block_time(1);
        ctx.env.set_caller(ctx.minter);
        let result = ctx.register
            .try_mint(ctx.user, String::from(TEST_LABEL), 0);
        assert_eq!(result.unwrap_err(), RegisterError::PastExpirationDate.into());
    }

    #[test]
    fn should_fail_when_sld_already_minted() {
        let mut ctx = RegisterTestContext::new();
        ctx.mint_token();

        ctx.env.set_caller(ctx.minter);
        let result = ctx.register
            .try_mint(ctx.user, String::from(TEST_LABEL), ctx.get_token_expiration());

        assert_eq!(result.unwrap_err(), CEP78Error::DuplicateIdentifier.into());
    }
}

mod bulk_mint {
    use crate::register::NameMintInfo;

    use super::*;
    #[test]
    fn should_mint_slds_in_bulk() {
        let mut ctx = RegisterTestContext::new();
        ctx.add_minter_role();

        let first_expiration_time = ctx.get_token_expiration();
        let first_token_id = blake2b(TEST_LABEL);
        let first_name = NameMintInfo::new(TEST_LABEL, first_expiration_time, ctx.user);

        let second_expiration_time = ctx.get_token_expiration();
        let second_token_label = "test-label-2";
        let second_token_id = blake2b(second_token_label);
        let second_name = NameMintInfo::new(second_token_label, second_expiration_time, ctx.user);

        ctx.env.set_caller(ctx.minter);
        ctx.register.bulk_mint(vec![first_name, second_name]);

        ctx.assert_token_minted(&first_token_id, first_expiration_time, TEST_LABEL);
        ctx.assert_token_minted(&second_token_id, second_expiration_time, second_token_label);
    }
}

#[test]
fn should_mint_renew_and_burn() {
    let mut ctx = RegisterTestContext::new();
    ctx.add_minter_role();
    ctx.add_operator_role();
    let token_id = String::from(TEST_LABEL_BLAKE2B);

    // Test mint.
    ctx.env.set_caller(ctx.minter);
    ctx.register
        .mint(ctx.user, String::from(TEST_LABEL), ONE_DAY);
    let event: Mint = ctx.register.get_event(-1).unwrap();
    let metadata = SLDMetadata::new(
        String::from(TEST_LABEL),
        ONE_DAY,
    );
    let expected = Mint::new(ctx.user, token_id.clone(), metadata.to_json().unwrap());
    assert_eq!(event, expected);

    // Test renew.
    ctx.register.renew(token_id.clone(), ONE_DAY * 2);
    let event: MetadataUpdated = ctx.register.get_event(-1).unwrap();
    let metadata = SLDMetadata::new(
        String::from(TEST_LABEL),
        ONE_DAY * 2,
    );
    let expected = MetadataUpdated::new(token_id.clone(), metadata.to_json().unwrap());
    assert_eq!(event, expected);

    ctx.env.set_caller(ctx.operator);
    ctx.register.admin_burn(token_id.clone());
    let event: Burn = ctx.register.get_event(-1).unwrap();
    let expected = Burn::new(ctx.user, token_id, ctx.operator);
    assert_eq!(event, expected);
}

#[test]
#[ignore]
fn test_metadata_serialization() {
    let expected = r#"{
        "token_hash": "44b7dfe6596e4668313215e4f12ee9650d911a59087944b077d5c087212b89b1",
        "to": "account-hash-88048a339fa20a4a17d746716aec4e1391be2ffdae4c60b1125320a0a07a3755",
        "label": "test-label",
        "expiration": 86400
    }"#.replace(" ", "").replace("\n", "");

    let metadata = SLDMetadata::new(
        String::from(TEST_LABEL),
        ONE_DAY
    );

    assert_eq!(expected, metadata.to_json().unwrap());

    let deserialized = SLDMetadata::from_json(&expected).unwrap();
    assert_eq!(metadata, deserialized);
}
