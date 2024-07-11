use std::io::Write;

use blake2::digest::VariableOutput;
use blake2::Blake2bVar;
use odra::args::Maybe;
use odra::casper_types::bytesrepr::{Bytes, ToBytes};
use odra::casper_types::U512;
use odra::host::{Deployer, HostEnv, HostRef};
use odra::{prelude::*, Address};
use odra_modules::cep78::events::Mint;

use crate::contracts::controller::{self, ControllerHostRef};
use crate::contracts::registrar::{RegistrarInitArgs, CONTROLLER_ROLE};
use crate::contracts::resolver::{DefaultResolverHostRef, DefaultResolverInitArgs};
use crate::contracts::{
    name_token::{NameTokenHostRef, NameTokenInitArgs},
    registrar::RegistrarHostRef,
};
use crate::data_structures::{NameMintInfo, NameTokenMetadata, TokenizationVoucher};

pub const NAME_TOKEN_NAME: &'static str = "NameToken";
pub const NAME_TOKEN_SYMBOL: &'static str = "NT";
pub const INIT_TIME: u64 = 1704103200000; // 2024-01-01T10:00:00.000Z
pub const ONE_DAY: u64 = 86400000;
pub const GRACE_PERIOD: u64 = ONE_DAY * 2;
pub const TOKEN_EXPIRATION: u64 = ONE_DAY * 365;
pub const VOUCHER_EXPIRATION: u64 = ONE_DAY * 7;
pub const TOKEN_HASH: &str = "label";

pub struct TestContext {
    pub env: HostEnv,
    pub token: NameTokenHostRef,
    pub registrar: RegistrarHostRef,
    pub controller: ControllerHostRef,
    pub default_resolver: DefaultResolverHostRef,
    pub admin: Address,
    pub alice: Address,
    pub bob: Address,
    pub anyone: Address,
    pub signer: Address,
    pub treasury: Address,
}

impl TestContext {
    pub fn install_raw() -> TestContext {
        let env = odra_test::env();
        let signer = env.get_account(10);
        let treasury = env.get_account(11);

        let name_token = NameTokenHostRef::deploy(
            &env,
            NameTokenInitArgs {
                name: String::from(NAME_TOKEN_NAME),
                symbol: String::from(NAME_TOKEN_SYMBOL),
            },
        );
        let resolver = DefaultResolverHostRef::deploy(
            &env,
            DefaultResolverInitArgs {
                name_token: *name_token.address(),
            },
        );
        let registrar = RegistrarHostRef::deploy(
            &env,
            RegistrarInitArgs {
                name_token: *name_token.address(),
                default_resolver: *resolver.address(),
            },
        );
        let controller = ControllerHostRef::deploy(
            &env,
            controller::ControllerInitArgs {
                registrar: *registrar.address(),
                signer: env.public_key(&signer),
                treasury,
            },
        );

        // Set start time.
        env.advance_block_time(INIT_TIME);

        TestContext {
            env: env.clone(),
            signer,
            token: name_token,
            registrar,
            controller,
            default_resolver: resolver,
            admin: env.get_account(0),
            alice: env.get_account(1),
            bob: env.get_account(2),
            anyone: env.get_account(3),
            treasury,
        }
    }

    pub fn install_and_setup() -> TestContext {
        let mut contracts = TestContext::install_raw();

        // Setup access.
        contracts.whitelist_registrar_in_name_token();
        contracts.set_controller_in_registrar();

        // Setup grace period.
        contracts.registrar.set_grace_period(GRACE_PERIOD);

        contracts
    }

    pub fn whitelist_registrar_in_name_token(&mut self) {
        self.token.set_variables(
            Maybe::Some(true),
            Maybe::Some(vec![*self.registrar.address()]),
            Maybe::None,
        );
    }

    pub fn set_controller_in_registrar(&mut self) {
        self.registrar
            .grant_role(&CONTROLLER_ROLE, self.controller.address());
    }

    pub fn sign<T: ToBytes>(&self, data: &T) -> Bytes {
        let bytes: Bytes = data.to_bytes().unwrap().into();
        self.env.sign_message(&bytes, &self.signer)
    }

    pub fn try_name_register(
        &mut self,
        caller: Address,
        recipient: Address,
        token_hash: &str,
        token_expiration: u64,
        voucher_expiration: u64,
    ) -> odra::OdraResult<()> {
        let names = vec![NameMintInfo::new(token_hash, recipient, token_expiration)];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);
        self.set_caller(caller);
        self.registrar.try_register(voucher)
    }

    pub fn with_name_registered(&mut self, caller: Address, recipient: Address, token_hash: &str) {
        let token_expiration = self.token_expiration_time();
        let voucher_expiration = self.voucher_expiration_time();
        self.try_name_register(
            caller,
            recipient,
            token_hash,
            token_expiration,
            voucher_expiration,
        )
        .unwrap()
    }

    pub fn expect_name_is_registered(&self, owner: Address, token_hash: &str) {
        let token_id = blake2b(token_hash);
        assert!(self.token.token_exists(&token_id), "Token does not exist");

        let actual_owner = self
            .token
            .owner_of(Maybe::None, Maybe::Some(token_id.clone()));
        assert_eq!(actual_owner, owner, "Owner is not correct");

        let metadata = self.token.metadata_by_hash(&token_id);
        let expected_metadata = NameTokenMetadata::with_resolver(
            token_hash,
            self.token_expiration_time(),
            *self.default_resolver.address(),
        );
        assert_eq!(metadata, expected_metadata);

        assert!(
            self.env.emitted_event(
                &self.token,
                &Mint::new(owner, token_id, expected_metadata.to_json().unwrap())
            ),
            "Mint event not emitted"
        );
    }

    pub fn try_name_expire(&mut self, token_hash: &str) -> odra::OdraResult<()> {
        let token_id = blake2b(token_hash);
        self.set_caller(self.anyone);
        self.registrar.try_expire(vec![token_id])
    }

    pub fn with_name_expired(&mut self, token_hash: &str) {
        self.try_name_expire(token_hash).unwrap()
    }

    pub fn token_expiration_time(&self) -> u64 {
        self.env.block_time() + TOKEN_EXPIRATION
    }

    pub fn voucher_expiration_time(&self) -> u64 {
        self.env.block_time() + VOUCHER_EXPIRATION
    }

    pub fn admin_transfer(&mut self, recipient: Address, token_hashes: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar
            .admin_transfer(recipient, blake2b_vec(token_hashes))
    }

    pub fn admin_burn(&mut self, token_hashes: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar.admin_burn(blake2b_vec(token_hashes));
    }

    pub fn set_caller(&mut self, caller: Address) {
        self.env.set_caller(caller);
    }

    pub fn advance_block_time(&mut self, time: u64) {
        self.env.advance_block_time(time);
    }

    pub fn balance_of(&self, account: &Address) -> U512 {
        self.env.balance_of(account)
    }
}

pub fn blake2b<T: AsRef<[u8]>>(data: T) -> String {
    let mut result = [0u8; 32];
    let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
    let _ = hasher.write(data.as_ref());
    hasher
        .finalize_variable(&mut result)
        .expect("should copy hash to the result array");
    hex::encode(result)
}

pub fn blake2b_vec<T: AsRef<[u8]>>(data: Vec<T>) -> Vec<String> {
    data.into_iter().map(blake2b).collect()
}
