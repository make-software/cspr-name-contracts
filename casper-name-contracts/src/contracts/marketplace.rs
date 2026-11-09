use odra::{
    casper_types::{bytesrepr::Bytes, PublicKey},
    prelude::*,
    ContractRef,
};
use odra_modules::{access::Role, dapp::DappContractBase};

use crate::{
    contracts::{controller::ControllerError, token_id::ToTokenId},
    data_structures::SecondarySaleVoucher,
};

use super::{controller::BaseController, name_token::NameTokenContractRef};

/// Secondary market smart contract. It handles the secondary market operations.
#[odra::module]
pub struct SecondaryMarket {
    controller: SubModule<BaseController>,
    name_token: External<NameTokenContractRef>,
    /// Membership in the CSPR.name dapp. The NameToken is the registry.
    dapp: SubModule<DappContractBase>,
}

#[odra::module]
impl SecondaryMarket {
    delegate! {
        to self.controller {
            fn has_role(&self, role: &Role, address: &Address) -> bool;
            fn grant_role(&mut self, role: &Role, address: &Address);
            fn revoke_role(&mut self, role: &Role, address: &Address);
            fn set_signer_public_key(&mut self, signer: PublicKey);
            fn set_treasury(&mut self, treasury: Address);
            fn signer_public_key(&self) -> PublicKey;
            fn pause(&mut self);
            fn unpause(&mut self);
            fn is_paused(&self) -> bool;
        }

        to self.dapp {
            /// Returns the dapp registry the contract belongs to.
            fn get_dapp_registry(&self) -> Address;
        }
    }

    /// Initializes the secondary market with the signer public key, the treasury
    /// address and the name token contract address.
    pub fn init(&mut self, signer: PublicKey, treasury: Address, name_token: Address) {
        if !name_token.is_contract() {
            self.revert(ControllerError::ContractAddressExpected);
        }
        self.controller.init(signer, treasury);
        // The NameToken is also the dapp registry.
        self.name_token.set(name_token);
        self.dapp.set_dapp_registry(&name_token);
    }

    /// Runs once during an upgrade. Points the contract at its dapp registry,
    /// which is the NameToken contract.
    pub fn upgrade(&mut self) {
        let registry = *self.name_token.address();
        self.dapp.set_dapp_registry(&registry);
    }

    /// Admin only. Changes the dapp registry the contract belongs to.
    pub fn set_dapp_registry(&mut self, registry: &Address) {
        self.controller.assert_caller_is_admin();
        self.dapp.set_dapp_registry(registry);
    }

    /// Payable. Buys name tokens from the secondary market.
    #[odra(payable)]
    pub fn buy(&mut self, voucher: SecondarySaleVoucher, signature: Bytes) {
        self.controller.require_not_paused();
        self.controller.process_payment_voucher(&voucher, signature);
        for name in voucher.names {
            let token_id = self.token_id(name.label);
            let to = self.env().caller();
            let from = name.owner;
            self.name_token.transfer_from(from, to, token_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::contracts::marketplace::{SecondaryMarket, SecondaryMarketInitArgs};
    use odra::host::Deployer;

    #[test]
    fn deploy_fails_if_account_set_as_name_token() {
        let env = odra_test::env();
        let signer = env.get_account(10);
        let treasury = env.get_account(11);
        let result = SecondaryMarket::try_deploy(
            &env,
            SecondaryMarketInitArgs {
                signer: env.public_key(&signer),
                treasury,
                name_token: env.get_account(12), // Using an account instead of a contract address
            },
        );
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod dapp_tests {
    use super::*;
    use crate::test_context::TestContext;
    use odra::host::{Deployer, InstallConfig, NoArgs};
    use odra_modules::access::errors::Error as AccessControlError;

    fn deploy_market(ctx: &TestContext) -> SecondaryMarketHostRef {
        SecondaryMarket::deploy_with_cfg(
            &ctx.env,
            SecondaryMarketInitArgs {
                signer: ctx.env.public_key(&ctx.signer),
                treasury: ctx.treasury,
                name_token: ctx.token.address(),
            },
            InstallConfig::upgradable::<SecondaryMarket>(),
        )
    }

    #[test]
    fn test_dapp_registry_is_the_name_token() {
        let mut ctx = TestContext::install_and_setup();
        let token = ctx.token.address();
        let other = ctx.registrar.address();
        ctx.set_caller(ctx.admin);
        let mut market = deploy_market(&ctx);
        assert_eq!(market.get_dapp_registry(), token);

        // The NameToken owner can register it.
        ctx.set_caller(ctx.admin);
        ctx.token.add_dapp_contract(&market.address(), false);
        assert!(ctx.token.is_dapp_contract(&market.address()));

        // Only the admin may re-point the contract.
        ctx.set_caller(ctx.alice);
        assert_eq!(
            market.try_set_dapp_registry(&other).unwrap_err(),
            AccessControlError::MissingRole.into()
        );
        ctx.set_caller(ctx.admin);
        market.set_dapp_registry(&other);
        assert_eq!(market.get_dapp_registry(), other);
    }

    #[test]
    fn test_upgrade_points_at_the_name_token() {
        let mut ctx = TestContext::install_and_setup();
        let token = ctx.token.address();
        ctx.set_caller(ctx.admin);
        let market = deploy_market(&ctx);
        let signer = market.signer_public_key();

        ctx.set_caller(ctx.admin);
        let upgraded = SecondaryMarket::try_upgrade(&ctx.env, market.address(), NoArgs).unwrap();
        assert_eq!(upgraded.get_dapp_registry(), token);
        // Existing storage survived.
        assert_eq!(upgraded.signer_public_key(), signer);
    }
}
