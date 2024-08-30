use odra::{
    args::Maybe,
    casper_types::{bytesrepr::Bytes, PublicKey},
    prelude::*,
    Address, External, SubModule, UnwrapOrRevert,
};
use odra_modules::access::Role;

use crate::data_structures::SecondarySaleVoucher;

use super::{controller::BaseController, name_token::NameTokenContractRef, utils};

// TODO: on the diagrams is called D3Operator, shouldn't we change it?
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
        }
    }

    pub fn init(&mut self, signer: PublicKey, treasury: Address, name_token: Address) {
        self.controller.init(signer, treasury);
        self.name_token.set(name_token);
    }

    #[odra(payable)]
    pub fn buy(&mut self, voucher: SecondarySaleVoucher, signature: Bytes) {
        self.controller.process_payment_voucher(&voucher, signature);
        for name in voucher.names {
            let token_hash = self.compute_namehash(&name.label);
            let target_key = self.env().caller();
            let source_key = name.owner;
            self.name_token
                .transfer(Maybe::None, Maybe::Some(token_hash), source_key, target_key);
        }
    }

    fn compute_namehash(&self, label: &String) -> String {
        let hash = self.env().hash(label);
        utils::to_utf8_string(&hash).unwrap_or_revert(self)
    }
}
