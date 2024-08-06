use odra::{casper_types::U512, prelude::*, Address, OdraResult};
use serde_json::{json, Value};

#[odra::odra_error]
#[derive(Debug)]
pub enum NameTokenError {
    EmptyTLD = 1001,
    TLDNotSupported = 1002,
    PastExpirationDate = 1003,
    EmptyLabel = 1004,
    SLDDoesNotExist = 1005,
    SerializationError = 1006,
    DeserializationError = 1007,
}

#[derive(PartialEq, Debug, Clone)]
pub struct NameTokenMetadata {
    value: Value,
}

impl NameTokenMetadata {
    pub fn with_resolver(name: &str, expiration: u64, resolver: Address) -> Self {
        Self {
            value: json!({
                "name": name,
                "expiration": expiration,
                "resolver": resolver
            }),
        }
    }

    pub fn with_no_resolver(name: &str, expiration: u64) -> Self {
        Self {
            value: json!({
                "name": name,
                "expiration": expiration,
                "resolver": null
            }),
        }
    }

    pub fn set_resolver(&mut self, resolver: Address) {
        self.value["resolver"] = json!(resolver);
    }

    pub fn resolver(&self) -> OdraResult<Option<Address>> {
        let resolver = self.value["resolver"].as_str();
        if let Some(resolver) = resolver {
            return Address::from_str(resolver)
                .map_err(|_| NameTokenError::DeserializationError.into())
                .map(Some);
        }
        Ok(None)
    }

    pub fn clear_resolver(&mut self) {
        self.value["resolver"] = json!(null);
    }

    pub fn json(&self) -> String {
        self.value.to_string()
    }

    pub fn expiration(&self) -> OdraResult<u64> {
        self.value["expiration"]
            .as_u64()
            .ok_or(NameTokenError::DeserializationError.into())
    }

    pub fn set_expiration(&mut self, expiration: u64) {
        self.value["expiration"] = json!(expiration);
    }
}

impl TryFrom<String> for NameTokenMetadata {
    type Error = NameTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Value::from_str(&value)
            .map(|value| Self { value })
            .map_err(|_| NameTokenError::DeserializationError)
    }
}

#[odra::odra_type]
pub struct PaymentInfo {
    pub buyer: Address,
    pub payment_id: String,
    pub amount: U512,
}

#[odra::odra_type]
pub struct TokenizationVoucher {
    pub names: Vec<NameMintInfo>,
    pub voucher_expiration: u64,
}

impl TokenizationVoucher {
    pub fn new(names: Vec<NameMintInfo>, voucher_expiration: u64) -> Self {
        Self {
            names,
            voucher_expiration,
        }
    }
}

#[odra::odra_type]
pub struct PaymentVoucher {
    pub payment: PaymentInfo,
    pub names: Vec<NameMintInfo>,
    pub voucher_expiration: u64,
}

impl PaymentVoucher {
    pub fn new(
        amount: U512,
        payment_id: &str,
        buyer: Address,
        names: Vec<NameMintInfo>,
        voucher_expiration: u64,
    ) -> Self {
        Self {
            payment: PaymentInfo {
                buyer,
                payment_id: String::from(payment_id),
                amount,
            },
            names,
            voucher_expiration,
        }
    }
}

impl From<PaymentVoucher> for TokenizationVoucher {
    fn from(voucher: PaymentVoucher) -> Self {
        Self {
            names: voucher.names,
            voucher_expiration: voucher.voucher_expiration,
        }
    }
}

#[odra::odra_type]
pub struct SecondarySaleVoucher {
    pub payment: PaymentInfo,
    pub names: Vec<NameTransferInfo>,
    pub voucher_expiration: u64,
}

#[odra::odra_type]
pub struct NameTransferInfo {
    pub label: String,
    pub owner: Address,
}

#[odra::odra_type]
pub struct NameMintInfo {
    pub label: String,
    pub owner: Address,
    pub token_expiration: u64,
}

impl NameMintInfo {
    pub fn new(label: &str, owner: Address, token_expiration: u64) -> Self {
        Self {
            label: String::from(label),
            owner,
            token_expiration,
        }
    }
}

#[odra::odra_type]
pub struct TokenRenewalInfo {
    pub token_id: String,
    pub token_expiration: u64,
}

impl TokenRenewalInfo {
    pub fn new(token_id: String, token_expiration: u64) -> Self {
        Self {
            token_id,
            token_expiration,
        }
    }
}

#[odra::odra_type]
pub struct RenewalPaymentVoucher {
    pub payment: PaymentInfo,
    pub tokens: Vec<TokenRenewalInfo>,
    pub voucher_expiration: u64,
}

impl RenewalPaymentVoucher {
    pub fn new(
        amount: U512,
        payment_id: &str,
        buyer: Address,
        tokens: Vec<TokenRenewalInfo>,
        voucher_expiration: u64,
    ) -> Self {
        Self {
            payment: PaymentInfo {
                buyer,
                payment_id: String::from(payment_id),
                amount,
            },
            tokens,
            voucher_expiration,
        }
    }
}

#[odra::odra_type]
pub struct RenewalVoucher {
    pub tokens: Vec<TokenRenewalInfo>,
    pub voucher_expiration: u64,
}

impl RenewalVoucher {
    pub fn new(tokens: Vec<TokenRenewalInfo>, voucher_expiration: u64) -> Self {
        Self {
            tokens,
            voucher_expiration,
        }
    }
}

impl From<RenewalPaymentVoucher> for RenewalVoucher {
    fn from(voucher: RenewalPaymentVoucher) -> Self {
        Self {
            tokens: voucher.tokens,
            voucher_expiration: voucher.voucher_expiration,
        }
    }
}

pub trait Payment {
    fn payment_info(&self) -> &PaymentInfo;
}

impl Payment for PaymentVoucher {
    fn payment_info(&self) -> &PaymentInfo {
        &self.payment
    }
}

impl Payment for RenewalPaymentVoucher {
    fn payment_info(&self) -> &PaymentInfo {
        &self.payment
    }
}

impl Payment for SecondarySaleVoucher {
    fn payment_info(&self) -> &PaymentInfo {
        &self.payment
    }
}

pub trait ExpirableVoucher {
    fn expiration_time(&self) -> u64;
}

impl ExpirableVoucher for TokenizationVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

impl ExpirableVoucher for RenewalVoucher {
    fn expiration_time(&self) -> u64 {
        self.voucher_expiration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization() {
        let expected = r#"{
            "name": "test-label",
            "expiration": 86400,
            "resolver": null
        }"#
        .replace(" ", "")
        .replace("\n", "");

        // Test metadata to_json.
        let metadata = NameTokenMetadata::with_no_resolver("test-label", 86400);
        assert_eq!(expected, metadata.json());

        // Test metadata from_json.
        let deserialized: NameTokenMetadata = expected.try_into().unwrap();
        assert_eq!(metadata, deserialized);

        let expected = r#"{
            "name": "test-label",
            "expiration": 86400,
            "resolver": "hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a"
        }"#
        .replace(" ", "")
        .replace("\n", "");

        // Test metadata to_json.
        let metadata = NameTokenMetadata::with_resolver(
            "test-label",
            86400,
            Address::new("hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a")
                .unwrap(),
        );
        assert_eq!(expected, metadata.json());

        // Test metadata from_json.
        let deserialized: NameTokenMetadata = expected.try_into().unwrap();
        assert_eq!(metadata, deserialized);
    }
}
