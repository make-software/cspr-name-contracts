use std::io::Write;

use blake2::digest::VariableOutput;
use blake2::Blake2bVar;
use odra::args::Maybe;
use odra::casper_types::bytesrepr::Bytes;
use odra::host::{Deployer, HostEnv, HostRef};
use odra::{prelude::*, Address};

use crate::contracts::controller::{self, ControllerHostRef};
use crate::contracts::registrar::RegistrarInitArgs;
use crate::contracts::{
    name_token::{NameTokenHostRef, NameTokenInitArgs},
    registrar::RegistrarHostRef,
};
use crate::data_structures::{NameTokenMetadata, TokenizationVoucher};

pub const NAME_TOKEN_NAME: &'static str = "NameToken";
pub const NAME_TOKEN_SYMBOL: &'static str = "NT";
pub const INIT_TIME: u64 = 1704103200000; // 2024-01-01T10:00:00.000Z
pub const ONE_DAY: u64 = 86400000;
pub const GRACE_PERIOD: u64 = ONE_DAY * 2;
pub const EXPIRATION: u64 = ONE_DAY * 365;

pub struct TestContext {
    pub env: HostEnv,
    pub signer: Address,
    pub token: NameTokenHostRef,
    pub registrar: RegistrarHostRef,
    pub controller: ControllerHostRef,
    pub admin: Address,
    pub alice: Address,
    pub bob: Address,
    pub anyone: Address,
}

impl TestContext {
    pub fn install_raw() -> TestContext {
        let env = odra_test::env();
        let signer = env.get_account(10);

        let name_token = NameTokenHostRef::deploy(
            &env,
            NameTokenInitArgs {
                name: String::from(NAME_TOKEN_NAME),
                symbol: String::from(NAME_TOKEN_SYMBOL),
            },
        );
        let registrar = RegistrarHostRef::deploy(
            &env,
            RegistrarInitArgs {
                name_token: name_token.address().clone(),
            },
        );
        let controller = ControllerHostRef::deploy(
            &env,
            controller::ControllerInitArgs {
                registrar: registrar.address().clone(),
                public_key: env.public_key(&signer),
            },
        );

        let contracts = TestContext {
            env: env.clone(),
            signer,
            token: name_token,
            registrar,
            controller,
            admin: env.get_account(0),
            alice: env.get_account(1),
            bob: env.get_account(2),
            anyone: env.get_account(3),
        };

        // Set start time.
        env.advance_block_time(INIT_TIME);

        contracts
    }

    pub fn install_and_setup() -> TestContext {
        let mut contracts = TestContext::install_raw();

        // Setup access.
        contracts.whitelist_registrar_in_name_token();

        // Setup grace period.
        contracts.registrar.set_grace_period(GRACE_PERIOD);

        contracts
    }

    pub fn whitelist_registrar_in_name_token(&mut self) {
        self.token.set_variables(
            Maybe::Some(true),
            Maybe::Some(vec![self.registrar.address().clone()]),
            Maybe::None,
        );
    }

    pub fn sign(&self, data: &Bytes) -> Bytes {
        self.env.sign_message(data, &self.signer)
    }

    pub fn try_name_register(
        &mut self,
        caller: &Address,
        recipient: &Address,
        label: &str,
        expiration: u64,
    ) -> odra::OdraResult<()> {
        let voucher = TokenizationVoucher::new(label, expiration, *recipient);
        self.env.set_caller(*caller);
        self.registrar.try_register(voucher)
    }

    pub fn with_name_registered(&mut self, caller: &Address, recipient: &Address, label: &str) {
        self.try_name_register(caller, recipient, label, self.expiration_time())
            .unwrap()
    }

    // TODO: Add more checks.
    pub fn expect_name_is_registered(&self, owner: &Address, label: &str) {
        let token_id = blake2b(label);
        assert!(self.token.token_exists(&token_id), "Token does not exist");
        let addr = self
            .token
            .owner_of(Maybe::None, Maybe::Some(token_id.clone()));
        assert_eq!(&addr, owner, "Owner is not correct");

        let metadata = self.token.metadata_by_hash(&token_id);
        let expected = NameTokenMetadata::new("test", self.expiration_time());
        assert_eq!(metadata, expected);
    }

    pub fn try_name_expire(&mut self, label: &str) -> odra::OdraResult<()> {
        let token_id = blake2b(label);
        self.env.set_caller(self.anyone);
        self.registrar.try_expire(vec![token_id])
    }

    pub fn with_name_expired(&mut self, label: &str) {
        self.try_name_expire(label).unwrap()
    }

    pub fn expiration_time(&self) -> u64 {
        self.env.block_time() + EXPIRATION
    }

    pub fn admin_transfer(&mut self, recipient: &Address, token_hashes: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar
            .admin_transfer(recipient.clone(), blake2b_vec(token_hashes))
    }

    pub fn admin_burn(&mut self, token_hashes: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar.admin_burn(blake2b_vec(token_hashes));
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
