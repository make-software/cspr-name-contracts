use odra::{
    casper_types::{U256, U512},
    prelude::*,
};
use serde::{Deserialize, Serialize};

/// Errors that can occur while working with name tokens.
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
    InvalidMetadata = 1008,
}

/// Metadata associated with a name token.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NameTokenMetadata {
    name: String,
    expiration: u64,
    resolver: Option<Address>,
    asset_uri: String,
}

impl NameTokenMetadata {
    pub fn with_resolver(name: &str, expiration: u64, asset_uri: &str, resolver: Address) -> Self {
        Self {
            name: String::from(name),
            expiration,
            resolver: Some(resolver),
            asset_uri: String::from(asset_uri),
        }
    }

    pub fn with_no_resolver(name: &str, expiration: u64, asset_uri: &str) -> Self {
        Self {
            name: String::from(name),
            expiration,
            resolver: None,
            asset_uri: String::from(asset_uri),
        }
    }

    pub fn set_resolver(&mut self, resolver: Address) {
        self.resolver = Some(resolver);
    }

    pub fn resolver(&self) -> OdraResult<Option<Address>> {
        Ok(self.resolver.clone())
    }

    pub fn clear_resolver(&mut self) {
        self.resolver = None;
    }

    pub fn json(&self) -> String {
        serde_json_wasm::to_string(&self).unwrap()
    }

    pub fn to_vec(&self) -> Vec<(String, String)> {
        let mut vec = Vec::new();
        vec.push(("asset_uri".to_string(), self.asset_uri.clone()));
        vec.push(("expiration".to_string(), self.expiration.to_string()));
        vec.push(("name".to_string(), self.name.clone()));
        if let Some(resolver) = &self.resolver {
            vec.push(("resolver".to_string(), resolver.to_string()));
        }
        vec
    }

    pub fn expiration(&self) -> OdraResult<u64> {
        Ok(self.expiration)
    }

    pub fn set_expiration(&mut self, expiration: u64) {
        self.expiration = expiration;
    }
}

impl TryFrom<String> for NameTokenMetadata {
    type Error = NameTokenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json_wasm::from_str(&value).map_err(|_| NameTokenError::DeserializationError)
    }
}

impl TryFrom<Vec<(String, String)>> for NameTokenMetadata {
    type Error = NameTokenError;

    fn try_from(value: Vec<(String, String)>) -> Result<Self, Self::Error> {
        let name = value
            .iter()
            .find(|(key, _)| key == "name")
            .ok_or(NameTokenError::DeserializationError)?
            .1
            .clone();

        let expiration = value
            .iter()
            .find(|(key, _)| key == "expiration")
            .ok_or(NameTokenError::DeserializationError)?
            .1
            .parse()
            .map_err(|_| NameTokenError::DeserializationError)?;

        let resolver = value
            .iter()
            .find(|(key, _)| key == "resolver")
            .cloned()
            .map(|(_, value)| {
                Address::from_str(&value).map_err(|_| NameTokenError::DeserializationError)
            })
            .transpose()?;

        let asset_uri = value
            .iter()
            .find(|(key, _)| key == "asset_uri")
            .map(|(_, value)| value.clone())
            .unwrap_or_default();

        Ok(NameTokenMetadata {
            name,
            expiration,
            resolver,
            asset_uri,
        })
    }
}

/// Information about a payment.
#[odra::odra_type]
pub struct PaymentInfo {
    pub buyer: Address,
    pub payment_id: String,
    pub amount: U512,
}

/// List of [NameMintInfo] structs and the expiration time of the voucher.
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

/// Information about a payment and a list of [NameMintInfo] structs.
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

/// Information about a payment and a list of [NameTransferInfo] structs.
#[odra::odra_type]
pub struct SecondarySaleVoucher {
    pub payment: PaymentInfo,
    pub names: Vec<NameTransferInfo>,
    pub voucher_expiration: u64,
}

/// Pair of a label and owner address.
#[odra::odra_type]
pub struct NameTransferInfo {
    pub label: String,
    pub owner: Address,
}

/// Basic minting information for a name token.
#[odra::odra_type]
pub struct NameMintInfo {
    pub label: String,
    pub owner: Address,
    pub token_expiration: u64,
    pub asset_uri: String,
}

impl NameMintInfo {
    pub fn new(label: &str, owner: Address, token_expiration: u64, asset_uri: &str) -> Self {
        Self {
            label: String::from(label),
            owner,
            token_expiration,
            asset_uri: String::from(asset_uri),
        }
    }
}

/// Renewal information with new expiration time.
#[odra::odra_type]
pub struct TokenRenewalInfo {
    pub token_id: U256,
    pub token_expiration: u64,
}

impl TokenRenewalInfo {
    pub fn new(token_id: U256, token_expiration: u64) -> Self {
        Self {
            token_id,
            token_expiration,
        }
    }
}

/// Voucher for renewing multiple name tokens, plus payment information.
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

/// Voucher for renewing multiple name tokens.
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
            "resolver": null,
            "asset_uri": ""
        }"#
        .replace(" ", "")
        .replace("\n", "");

        // Test metadata to_json.
        let metadata = NameTokenMetadata::with_no_resolver("test-label", 86400, "");
        assert_eq!(expected, metadata.json());

        // Test metadata from_json.
        let deserialized: NameTokenMetadata = expected.try_into().unwrap();
        assert_eq!(metadata, deserialized);

        let expected = r#"{
            "name": "test-label",
            "expiration": 86400,
            "resolver": "hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a",
            "asset_uri": "https://example.com/asset-uri"
        }"#
        .replace(" ", "")
        .replace("\n", "");

        // Test metadata to_json.
        let metadata = NameTokenMetadata::with_resolver(
            "test-label",
            86400,
            "https://example.com/asset-uri",
            Address::new("hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a")
                .unwrap(),
        );
        assert_eq!(expected, metadata.json());

        // Test metadata from_json.
        let deserialized: NameTokenMetadata = expected.try_into().unwrap();
        assert_eq!(metadata, deserialized);
    }
}

//"{\"name\":\"test-label\",\"expiration\":86400,\"resolver\":\"hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a\",\"asset_uri\":\"https://example.com/asset-uri\"}"
//"{\"name\":\"test-label\",\"expiration\":86400,\"resolver\":\"hash-7ba9daac84bebee8111c186588f21ebca35550b6cf1244e71768bd871938be6a\"\"asset_uri\":\"https://example.com/asset-uri\"}"
