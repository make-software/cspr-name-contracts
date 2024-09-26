use casper_name_contracts::data_structures::NameMintInfo;
use odra::{casper_types::{bytesrepr::ToBytes, CLTyped}, Address};

pub fn main() {
    let names: Vec<NameMintInfo> = vec![
        NameMintInfo {
            label: String::from("my_token_1"),
            owner: Address::new(
                "account-hash-3a4751fd7ae4ad2060e68667b395f217af21ebb41780ea4ead07b64ce2e33be0",
            )
            .unwrap(),
            token_expiration: 6401000000000000,
        }
    ];

    println!("Type:\n{:?}\n", Vec::<NameMintInfo>::cl_type());
    
    let bytes = names.to_bytes().unwrap();
    println!("Full bytes:\n{}\n", hex::encode(bytes));

    let bytes = names[0].to_bytes().unwrap();
    println!("Single struct:\n{}\n", hex::encode(bytes));

    let bytes = names[0].label.to_bytes().unwrap();
    println!("Label:\n{}\n", hex::encode(bytes));

    let bytes = names[0].owner.to_bytes().unwrap();
    println!("Owner:\n{}\n", hex::encode(bytes));

    let bytes = names[0].token_expiration.to_bytes().unwrap();
    println!("Token expiration:\n{}\n", hex::encode(bytes));
}