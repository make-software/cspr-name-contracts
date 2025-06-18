use odra::{
    casper_types::{bytesrepr::Bytes, PublicKey, U256},
    prelude::*,
};
use odra_modules::access::Role;

use crate::data_structures::SecondarySaleVoucher;

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
        self.controller.init(signer, treasury);
        self.name_token.set(name_token);
    }

    /// Payable. Buys name tokens from the secondary market.
    #[odra(payable)]
    pub fn buy(&mut self, voucher: SecondarySaleVoucher, signature: Bytes) {
        self.controller.require_not_paused();
        self.controller.process_payment_voucher(&voucher, signature);
        for name in voucher.names {
            let token_id = self.compute_token_id(&name.label);
            let to = self.env().caller();
            let from = name.owner;
            self.name_token.transfer_from(from, to, token_id);
        }
    }

    fn compute_token_id(&self, label: &String) -> U256 {
        let hash = self.env().hash(label);
        U256::from(hash)
    }
}
