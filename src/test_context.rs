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

pub const NAME_TOKEN_NAME: &'static str = "NameToken";
pub const NAME_TOKEN_SYMBOL: &'static str = "NT";
pub const INIT_TIME: u64 = 1704103200000; // 2024-01-01T10:00:00.000Z

pub struct TestContext {
    pub env: HostEnv,
    pub signer: Address,
    pub name_token: NameTokenHostRef,
    pub registrar: RegistrarHostRef,
    pub controller: ControllerHostRef,
}

impl TestContext {
    pub fn install(env: &HostEnv) -> TestContext {
        let signer = env.get_account(10);

        let name_token = NameTokenHostRef::deploy(
            env,
            NameTokenInitArgs {
                name: String::from(NAME_TOKEN_NAME),
                symbol: String::from(NAME_TOKEN_SYMBOL),
            },
        );
        let registrar = RegistrarHostRef::deploy(
            env,
            RegistrarInitArgs {
                name_token: name_token.address().clone(),
            },
        );
        let controller = ControllerHostRef::deploy(
            env,
            controller::ControllerInitArgs {
                registrar: registrar.address().clone(),
                public_key: env.public_key(&signer),
            },
        );

        let mut contracts = TestContext {
            env: env.clone(),
            signer,
            name_token,
            registrar,
            controller,
        };

        // Setup contracts.
        contracts.whitelist_registrar_in_name_token();

        // Advance time.
        env.advance_block_time(INIT_TIME);

        contracts
    }

    pub fn whitelist_registrar_in_name_token(&mut self) {
        self.name_token.set_variables(
            Maybe::Some(true),
            Maybe::Some(vec![self.registrar.address().clone()]),
            Maybe::None,
        );
    }

    pub fn sign(&self, data: &Bytes) -> Bytes {
        self.env.sign_message(data, &self.signer)
    }
}

// fn blake2b<T: AsRef<[u8]>>(data: T) -> String {
//     let mut result = [0u8; 32];
//     let mut hasher = <Blake2bVar as VariableOutput>::new(32).expect("should create hasher");
//     let _ = hasher.write(data.as_ref());
//     hasher
//         .finalize_variable(&mut result)
//         .expect("should copy hash to the result array");
//     hex::encode(result)
// }
