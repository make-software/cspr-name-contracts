use casper_name::name_token::NameToken;

fn main() {
    let events = <NameToken as odra::contract_def::HasEvents>::event_schemas();
    for event in events {
        println!("Event: {:?}", event);
    }
}