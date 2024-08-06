use odra::{
    casper_types::{bytesrepr::Bytes, PublicKey},
    prelude::*,
    Address, External, SubModule, UnwrapOrRevert,
};

use crate::data_structures::SecondarySaleVoucher;

use super::{
    controller::BaseController,
    name_token::NameTokenContractRef,
    utils,
};

#[odra::module]
struct SecondaryMarket {
    controller: SubModule<BaseController>,
    name_token: External<NameTokenContractRef>,
}

#[odra::module]
impl SecondaryMarket {
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
                .transfer_by_hash(token_hash, source_key, target_key);
            // self.name_token.metadata(token_id, token_hash);
            // self.name_token.set_token_metadata(token_id, resolver);
        }
    }

    fn compute_namehash(&self, label: &String) -> String {
        let hash = self.env().hash(label);
        utils::to_utf8_string(&hash).unwrap_or_revert(self)
    }
}
