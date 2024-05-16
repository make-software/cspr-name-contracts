use casper_name::register::Register;

fn main() {
    let events = <Register as odra::contract_def::HasEvents>::event_schemas();
    for event in events {
        println!("Event: {:?}", event);
    }
}