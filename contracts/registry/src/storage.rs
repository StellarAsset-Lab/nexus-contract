use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Asset(Address),
    Distribution(Address, Address),
    Eligibility(Address, Address, Address),
    Admin,
    PendingAdmin,
    Initialized,
}
