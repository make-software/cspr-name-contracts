use odra::prelude::*;

const CSPR_DOMAIN: &str = ".cspr";
const SUBDOMAIN_SEPARATOR: &str = ".";

/// Extract the token name from a full domain.
pub fn extract_token_name(full_domain: &str) -> Option<String> {
    if full_domain.ends_with(CSPR_DOMAIN) {
        let token_name = full_domain.trim_end_matches(CSPR_DOMAIN);
        token_name.split('.').last().map(|s| s.to_string())
    } else {
        None
    }
}

/// Return `true` if byte `b` is ASCII letter or digit.
#[inline(always)]
const fn is_alnum(b: u8) -> bool {
    (b >= b'0' && b <= b'9') || (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z')
}

/// Validate a single DNS label under RFC 952/1123 (LDH rule).
///
/// * **length** 1–63 bytes
/// * **characters** ASCII letters, digits, or `-`
/// * **first/last** must be alphanumeric
///
/// Returns `true` for valid labels.
#[inline]
fn is_valid_dns_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    let len = bytes.len();

    if len == 0 || len > 63 {
        return false;
    }

    // first & last char must be alnum
    if !is_alnum(bytes[0]) || !is_alnum(bytes[len - 1]) {
        return false;
    }

    // interior chars: alnum or hyphen
    let mut i = 1;
    while i < len - 1 {
        let b = bytes[i];
        if !is_alnum(b) && b != b'-' {
            return false;
        }
        i += 1;
    }
    true
}

/// Validate a full domain with subdomains.
/// The domain must end with `.cspr` and each subdomain must be a valid DNS label.
pub fn is_domain_valid(full_domain: &str) -> bool {
    if full_domain.ends_with(CSPR_DOMAIN) {
        let token_name = full_domain.trim_end_matches(CSPR_DOMAIN);
        token_name
            .split(SUBDOMAIN_SEPARATOR)
            .map(is_valid_dns_label)
            .all(|valid| valid)
    } else {
        false
    }
}

/// Validate a label for use in a Casper token name.
/// A valid label must be a valid DNS label, not equal to "cspr", and
/// must not contain any subdomains (i.e., no `.` character).
pub fn is_label_valid(label: &str) -> bool {
    is_valid_dns_label(label) && label != "cspr" && !label.contains('.')
}

/// Convert microsecond timestamps to milliseconds if they exceed the threshold.
///
/// The offchain component may supply timestamps in microseconds instead of
/// milliseconds. This helper is applied at every timestamp entry point
/// ([NameTokenMetadata](crate::data_structures::NameTokenMetadata) construction
/// and deserialization, voucher expiration reads, and registrar inputs) so all
/// time comparisons work in milliseconds, matching the block time.
pub fn trim_microseconds_to_milliseconds_if_needed(expiration: u64) -> u64 {
    // Values above ~10^14 are microseconds: 10^14 ms is the year 5138,
    // while any microsecond timestamp after 1973 exceeds it.
    if expiration > 99_999_999_999_999 {
        expiration / 1000
    } else {
        expiration
    }
}

#[cfg(test)]
mod t {
    #[test]
    fn test_extract_token_name() {
        let full_domain = "odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "aaa.odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "ss.aaa.odra.cspr";
        let token_name = super::extract_token_name(full_domain).unwrap();
        assert_eq!(token_name, "odra");

        let full_domain = "aaa.odra.csp";
        let token_name = super::extract_token_name(full_domain);
        assert_eq!(token_name, None);
    }

    #[test]
    fn test_is_valid_dns_label() {
        assert!(super::is_valid_dns_label("valid-label"));
        assert!(!super::is_valid_dns_label("-invalid-start"));
        assert!(!super::is_valid_dns_label("invalid-end-"));
        assert!(!super::is_valid_dns_label("invalid_char@"));
        assert!(!super::is_valid_dns_label(
            "too-long-label-abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz"
        ));
        assert!(super::is_valid_dns_label("valid123"));
    }

    #[test]
    fn test_is_domain_valid() {
        let full_domain = "odra.cspr";
        assert!(super::is_domain_valid(full_domain));

        let full_domain = "aaa.odra.cspr";
        assert!(super::is_domain_valid(full_domain));

        let full_domain = "ss.aaa.odra.cspr";
        assert!(super::is_domain_valid(full_domain));

        let full_domain = "invalid-label-.cspr";
        assert!(!super::is_domain_valid(full_domain));

        let full_domain = "invalid-label@.cspr";
        assert!(!super::is_domain_valid(full_domain));

        let full_domain =
            "too-long-label-abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz.cspr";
        assert!(!super::is_domain_valid(full_domain));

        let full_domain = "valid123.cspr";
        assert!(super::is_domain_valid(full_domain));
    }

    #[test]
    fn test_is_label_valid() {
        assert!(super::is_label_valid("valid-label"));
        assert!(!super::is_label_valid("-invalid-start"));
        assert!(!super::is_label_valid("invalid-end-"));
        assert!(!super::is_label_valid("invalid_char@"));
        assert!(!super::is_label_valid("cspr"));
        assert!(!super::is_label_valid("invalid.label"));
        assert!(super::is_label_valid("valid123"));
    }

    #[test]
    fn test_trim_microseconds_to_milliseconds_if_needed() {
        assert_eq!(super::trim_microseconds_to_milliseconds_if_needed(100_000_000_000_000), 100_000_000_000);
        assert_eq!(super::trim_microseconds_to_milliseconds_if_needed(99_999_999_999_999), 99_999_999_999_999);
    }
}
