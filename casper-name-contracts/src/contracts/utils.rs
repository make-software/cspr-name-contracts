use odra::ContractEnv;
use crate::data_structures::ExpirableVoucher;
use super::registrar::RegistrarError;

pub fn assert_voucher_not_expired<T: ExpirableVoucher>(e: &T, env: &ContractEnv) {
    if e.expiration_time() < env.get_block_time() {
        env.revert(RegistrarError::VoucherExpired);
    }
}