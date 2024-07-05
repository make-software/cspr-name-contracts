use super::registrar::RegistrarError;
use crate::data_structures::ExpirableVoucher;
use odra::module::Revertible;

pub fn assert_voucher_not_expired<T: ExpirableVoucher, R: Revertible>(
    expirable: &T,
    block_time: u64,
    revertible: &R,
) {
    if expirable.expiration_time() < block_time {
        revertible.revert(RegistrarError::VoucherExpired);
    }
}
