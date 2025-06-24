use odra::{
    casper_types::{bytesrepr::Bytes, PublicKey},
    prelude::*,
};
use odra_modules::access::Role;

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
    }

    /// Initializes the secondary market with the signer public key, the treasury
    /// address and the name token contract address.
    pub fn init(&mut self, signer: PublicKey, treasury: Address, name_token: Address) {
        if !name_token.is_contract() {
            self.revert(ControllerError::ContractAddressExpected);
        }
        self.controller.init(signer, treasury);
        self.name_token.set(name_token);
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
