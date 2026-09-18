use soroban_sdk::{contracttype, Address, Env};

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

pub const DAY_IN_LEDGERS: u32 = 17_280;
pub const TTL_BUMP_LEDGERS: u32 = 30 * DAY_IN_LEDGERS;
pub const TTL_THRESHOLD_LEDGERS: u32 = 29 * DAY_IN_LEDGERS;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_BUMP_LEDGERS);
}

pub fn extend_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD_LEDGERS, TTL_BUMP_LEDGERS);
}
