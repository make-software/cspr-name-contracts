use odra::{
    casper_types::{
        bytesrepr::{Bytes, ToBytes},
        PublicKey,
    },
    module::Module,
    Address, UnwrapOrRevert, Var,
};

use crate::data_structures::PaymentVoucher;

#[odra::module]
pub struct Controller {
    signer_public_key: Var<PublicKey>,
    registrar: Var<Address>,
}

#[odra::module]
impl Controller {
    pub fn init(&mut self, registrar: Address, public_key: PublicKey) {
        self.registrar.set(registrar);
        self.signer_public_key.set(public_key);
    }

    pub fn signer_public_key(&self) -> PublicKey {
        self.signer_public_key.get().unwrap_or_revert(&self.env())
    }

    pub fn buy(&mut self, voucher: PaymentVoucher, signature: Bytes) {
        let public_key = self.signer_public_key();
        let bytes: Bytes = voucher.to_bytes().unwrap_or_revert(&self.env()).into();
        let verified = self.env().verify_signature(&bytes, &signature, &public_key);
        if !verified {
            self.env().revert(ControllerError::InvalidSignature);
        }
    }
}

#[odra::odra_error]
pub enum ControllerError {
    InvalidSignature = 2001,
}

#[cfg(test)]
mod tests {
    use odra::casper_types::U512;

    use crate::test_context::TestContext;

    use super::*;

    #[test]
    fn test_controller() {
        let env = odra_test::env();
        let mut ctx = TestContext::install(&env);

        let alice = env.get_account(1);

        let voucher = PaymentVoucher::new("label", 100, alice, U512::from(2000));
        let bytes: Bytes = voucher.to_bytes().unwrap().into();
        let signature = ctx.sign(&bytes);

        ctx.controller.buy(voucher, signature.clone());

        let voucher2 = PaymentVoucher::new("label2", 100, alice, U512::from(2000));
        let bytes2: Bytes = voucher2.to_bytes().unwrap().into();
        let signature2 = ctx.sign(&bytes2);

        let _ = ctx.controller.try_buy(voucher2, signature);
    }
}
