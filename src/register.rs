use odra::prelude::*;
use odra::{casper_types::U256, Address, Mapping, SubModule, UnwrapOrRevert};
use odra_modules::erc721::extensions::erc721_metadata::Erc721Metadata;
use odra_modules::{
    access::{AccessControl, Role, DEFAULT_ADMIN_ROLE},
    erc721::{erc721_base::Erc721Base, extensions::erc721_metadata::Erc721MetadataExtension},
};

// TODO: Match roles from the Solidity code.
const MINTER_ROLE: Role = [1; 32];
const OPERATOR_ROLE: Role = [2; 32];

#[odra::odra_error]
pub enum RegisterError {
    EmptyTLD = 1,
    TLDNotSupported = 2,
    PastExpirationDate = 3,
    EmptyLabel = 4,
}

#[odra::event]
pub struct TLDAdded {
    tld: String,
}

#[odra::event]
pub struct TLDMinted {
    token_id: U256,
    to: Address,
    label: String,
    tld: String,
    expiration: u64,
}

#[odra::module]
pub struct Register {
    access_control: SubModule<AccessControl>,
    token: SubModule<Erc721Base>,
    metadata: SubModule<Erc721MetadataExtension>,

    /// Maps TLD name to its namehash.
    /// Used to check whether TLD exists.
    /// Also, its namehash is precomputed, so we save on gas costs.
    tlds: Mapping<String, U256>,

    /// Maps Token ID to its expiration date.
    /// Expiration date is a timestamp in seconds.
    expirations: Mapping<U256, u64>,
}

#[odra::module]
impl Register {
    pub fn init(&mut self, name: String, symbol: String, tlds: Vec<String>, uri: String) {
        let caller = self.env().caller();

        // Setup access control.
        self.access_control
            .unchecked_grant_role(&DEFAULT_ADMIN_ROLE, &caller);
        self.access_control
            .set_admin_role(&MINTER_ROLE, &DEFAULT_ADMIN_ROLE);
        self.access_control
            .set_admin_role(&OPERATOR_ROLE, &DEFAULT_ADMIN_ROLE);

        // Setup ERC721 token.
        self.metadata.init(name, symbol, uri);

        // Init TLDs.
        for tld in tlds {
            self.add_tld_unchecked(tld);
        }

        self.env().revert(RegisterError::EmptyLabel);
    }

    // TYPE_CHANGE: expiration from u256 to u64.
    // TODO: merge check and get.
    pub fn mint(&mut self, to: Address, label: String, tld: String, expiration: u64) {
        self.assert_minter_role();
        self.check_tld_supported(&tld);
        self.require_future_expiration_date(expiration);

        if label.is_empty() {
            self.env().revert(RegisterError::EmptyLabel);
        }

        let tld_hash = self.tlds.get(&tld).unwrap_or_revert(&self.env());
        let token_id = self.compute_namehash(tld_hash, &label);

        if self.token_exists(&token_id) && self.is_token_expired(&token_id) {
            self.burn_single(&token_id);
        }

        self.mint_single(to, &token_id);
        self.expirations.set(&token_id, expiration);

        self.env().emit_event(TLDMinted {
            token_id,
            to,
            label,
            tld,
            expiration,
        });
    }

    delegate! {
        to self.metadata {
            fn name(&self) -> String;
            fn symbol(&self) -> String;
            fn base_uri(&self) -> String;
        }

        to self.access_control {
            fn grant_role(&mut self, role: &Role, address: &Address);
            // TODO: Decide on role management public functions.
        }
    }
}

impl Register {
    fn add_tld_unchecked(&mut self, tld: String) {
        if tld.is_empty() {
            self.env().revert(RegisterError::EmptyTLD);
        }

        let namehash = self.compute_namehash(U256::zero(), &tld);
        self.tlds.set(&tld, namehash);

        self.env().emit_event(TLDAdded { tld });
    }

    // TODO: Make sure this implementation is sufficient.
    fn compute_namehash(&self, parent: U256, tld: &str) -> U256 {
        U256::from(self.env().hash((parent, tld)))
    }

    fn check_tld_supported(&self, tld: &String) {
        if self.tlds.get(tld).is_none() {
            self.env().revert(RegisterError::TLDNotSupported);
        }
    }

    fn require_future_expiration_date(&self, expiration: u64) {
        if expiration < self.env().get_block_time() {
            self.env().revert(RegisterError::PastExpirationDate);
        }
    }

    fn token_exists(&self, token_id: &U256) -> bool {
        self.token.exists(token_id)
    }

    fn is_token_expired(&self, token_id: &U256) -> bool {
        self.expirations.get_or_default(token_id) < self.env().get_block_time()
    }

    fn burn_single(&mut self, token_id: &U256) {
        use odra_modules::erc721::Erc721;
        let owner = self.token.owner_of(token_id);
        let balance = self.token.balance_of(&owner);
        self.token.balances.set(&owner, balance - U256::from(1));
        self.token.owners.set(token_id, None);
        self.token.clear_approval(token_id);
    }

    fn mint_single(&mut self, to: Address, token_id: &U256) {
        use odra_modules::erc721::Erc721;
        let balance = self.token.balance_of(&to);
        self.token.balances.set(&to, balance + U256::from(1));
        self.token.owners.set(token_id, Some(to));
    }

    fn assert_minter_role(&self) {
        self.access_control
            .check_role(&MINTER_ROLE, &self.env().caller());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostEnv, HostRef};

    const NFT_NAME: &'static str = "D3 Tokens";
    const NFT_SYMBOL: &'static str = "D3";
    const D3_TLD: &'static str = "d3";
    const TEST_TLD: &'static str = "test";
    const TEST_LABEL: &'static str = "test-label";
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
                    tlds: vec![String::from(D3_TLD), String::from(TEST_TLD)],
                    uri: String::from(BASE_URI),
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
    }

    mod initialize {

        use super::*;

        #[test]
        fn should_set_correct_name_and_symbol() {
            let ctx = RegisterTestContext::new();
            assert_eq!(ctx.register.name(), NFT_NAME);
            assert_eq!(ctx.register.symbol(), NFT_SYMBOL);
        }

        #[test]
        fn should_emit_correct_tld_events() {
            let ctx = RegisterTestContext::new();
            let event1: TLDAdded = ctx.register.get_event(-2).unwrap();
            let event2: TLDAdded = ctx.register.get_event(-1).unwrap();
            assert_eq!(
                event1,
                TLDAdded {
                    tld: String::from(D3_TLD)
                }
            );
            assert_eq!(
                event2,
                TLDAdded {
                    tld: String::from(TEST_TLD)
                }
            );
            // TODO: Assert access_control events.
            assert_eq!(ctx.env.events_count(ctx.register.address()), 5);
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
        use super::*;

        #[test]
        fn should_mint_sld_nft() {
            let mut ctx = RegisterTestContext::new();
            ctx.add_minter_role();
            
            ctx.env.set_caller(ctx.minter);
            ctx.register.mint(
                ctx.user,
                String::from(TEST_LABEL),
                String::from(TEST_TLD),
                ONE_DAY_SECONDS,
            );
            let event: TLDMinted = ctx.register.get_event(-1).unwrap();
            // assert_eq!(event.token_id, U256::from(1));
            assert_eq!(event.to, ctx.user);
            assert_eq!(event.label, TEST_LABEL);
            assert_eq!(event.tld, TEST_TLD);
            assert_eq!(event.expiration, ONE_DAY_SECONDS);
        }
    }
}
