#![no_std]

mod error;
mod events;
mod storage;
mod types;

use error::ContractError;
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::DataKey;

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(ContractError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        Ok(())
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn pending_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PendingAdmin)
    }
}
