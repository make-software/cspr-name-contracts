use std::io::Write;

use blake2::digest::VariableOutput;
use blake2::Blake2bVar;
use odra::casper_types::bytesrepr::{Bytes, ToBytes};
use odra::casper_types::{U256, U512};
use odra::host::{Deployer, HostEnv, HostRef};
use odra::prelude::*;
use odra_modules::access::DEFAULT_ADMIN_ROLE;
use odra_modules::cep95::Mint;

use crate::contracts::controller::{self, Controller, ControllerHostRef};
use crate::contracts::name_token::NameToken;
use crate::contracts::registrar::{Registrar, RegistrarInitArgs, CONTROLLER_ROLE};
use crate::contracts::resolver::{
    DefaultResolver, DefaultResolverHostRef, DefaultResolverInitArgs,
};
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
pub const TOKEN_NAME: &str = "label";

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

        let name_token = NameToken::deploy(
            &env,
            NameTokenInitArgs {
                name: String::from(NAME_TOKEN_NAME),
                symbol: String::from(NAME_TOKEN_SYMBOL),
            },
        );
        let resolver = DefaultResolver::deploy(
            &env,
            DefaultResolverInitArgs {
                name_token: *name_token.address(),
            },
        );
        let registrar = Registrar::deploy(
            &env,
            RegistrarInitArgs {
                name_token: *name_token.address(),
            },
        );
        let controller = Controller::deploy(
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

        // register default resolver
        contracts.whitelist_admin_in_name_token();
        contracts.register_default_resolver_in_name_token();
        // Setup access.
        contracts.whitelist_registrar_in_name_token();
        contracts.set_controller_in_registrar();
        contracts.set_name_token_in_resolver();

        // Setup grace period.
        contracts.registrar.set_grace_period(GRACE_PERIOD);

        contracts
    }

    pub fn whitelist_registrar_in_name_token(&mut self) {
        self.token.whitelist(*self.registrar.address());
    }

    pub fn whitelist_admin_in_name_token(&mut self) {
        self.token.whitelist(self.admin);
    }

    pub fn set_controller_in_registrar(&mut self) {
        self.registrar
            .grant_role(&CONTROLLER_ROLE, self.controller.address());
    }

    pub fn set_name_token_in_resolver(&mut self) {
        self.default_resolver
            .grant_role(&DEFAULT_ADMIN_ROLE, self.token.address());
    }

    pub fn register_default_resolver_in_name_token(&mut self) {
        let address = *self.default_resolver.address();
        self.token.set_default_resolver(address);
    }

    pub fn sign<T: ToBytes>(&self, data: &T) -> Bytes {
        let bytes: Bytes = data.to_bytes().unwrap().into();
        self.env.sign_message(&bytes, &self.signer)
    }

    pub fn try_name_register(
        &mut self,
        caller: Address,
        recipient: Address,
        token_name: &str,
        token_expiration: u64,
        voucher_expiration: u64,
    ) -> OdraResult<()> {
        let names = vec![NameMintInfo::new(
            token_name,
            recipient,
            token_expiration,
            "",
        )];
        let voucher = TokenizationVoucher::new(names, voucher_expiration);
        self.set_caller(caller);
        self.registrar.try_controller_register(voucher)
    }

    pub fn with_name_registered(&mut self, caller: Address, recipient: Address, token_name: &str) {
        let token_expiration = self.token_expiration_time();
        let voucher_expiration = self.voucher_expiration_time();
        self.try_name_register(
            caller,
            recipient,
            token_name,
            token_expiration,
            voucher_expiration,
        )
        .unwrap()
    }

    pub fn with_multi_names_registered(
        &mut self,
        caller: Address,
        recipient: Address,
        token_names: Vec<&str>,
    ) {
        let token_expiration = self.token_expiration_time();
        let voucher_expiration = self.voucher_expiration_time();
        let names = token_names
            .iter()
            .map(|label| NameMintInfo::new(*label, recipient, token_expiration, ""))
            .collect();
        let voucher = TokenizationVoucher::new(names, voucher_expiration);
        self.set_caller(caller);
        self.registrar.try_controller_register(voucher).unwrap()
    }

    pub fn expect_name_is_registered(&self, owner: Address, token_name: &str) {
        let token_id = generate_token_id(token_name);
        assert!(self.token.token_exists(token_id), "Token does not exist");

        let actual_owner = self.token.owner_of(token_id);
        assert_eq!(actual_owner, Some(owner), "Owner is not correct");

        let metadata = self.token.token_metadata(token_id.clone());
        let expected_metadata = NameTokenMetadata::with_resolver(
            token_name,
            self.token_expiration_time(),
            "",
            *self.default_resolver.address(),
        );
        assert_eq!(metadata, expected_metadata.to_vec());

        assert!(
            self.env.emitted_event(
                &self.token,
                &Mint {
                    to: owner,
                    token_id
                }
            ),
            "Mint event not emitted"
        );
    }

    pub fn try_name_expire(&mut self, token_name: &str) -> OdraResult<()> {
        let token_id = generate_token_id(token_name);
        self.set_caller(self.anyone);
        self.registrar.try_expire(vec![token_id])
    }

    pub fn with_name_expired(&mut self, token_name: &str) {
        self.try_name_expire(token_name).unwrap()
    }

    pub fn with_names_expired(&mut self, token_names: Vec<&str>) {
        let tokens_ids = token_names.iter().map(generate_token_id).collect();
        self.set_caller(self.anyone);
        self.registrar.try_expire(tokens_ids).unwrap();
    }

    pub fn token_expiration_time(&self) -> u64 {
        self.env.block_time() + TOKEN_EXPIRATION
    }

    pub fn voucher_expiration_time(&self) -> u64 {
        self.env.block_time() + VOUCHER_EXPIRATION
    }

    pub fn admin_transfer(&mut self, recipient: Address, token_names: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar
            .admin_transfer(recipient, generate_token_id_vec(token_names))
    }

    pub fn admin_burn(&mut self, token_names: Vec<&str>) {
        self.env.set_caller(self.admin);
        self.registrar
            .admin_burn(generate_token_id_vec(token_names));
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

pub fn generate_token_id<T: AsRef<[u8]>>(data: T) -> U256 {
    let mut result = [0u8; 32];
    let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
    let _ = hasher.write(data.as_ref());
    hasher
        .finalize_variable(&mut result)
        .expect("should copy hash to the result array");
    U256::from(result)
}

pub fn generate_token_id_vec<T: AsRef<[u8]>>(data: Vec<T>) -> Vec<U256> {
    data.into_iter().map(generate_token_id).collect()
}
