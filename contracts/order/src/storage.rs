use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    PendingAdmin,
    Initialized,
    Registry,
    Paused,
    NextOrderId,
    Order(u64),
}
