#![no_std]

mod error;
mod events;
mod storage;
mod types;

use error::ContractError;
use events::{AdminTransferCancelled, AdminTransferProposed, AdminTransferred};
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

    pub fn propose_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        if env.storage().instance().has(&DataKey::PendingAdmin) {
            return Err(ContractError::PendingAdminAlreadySet);
        }

        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);

        AdminTransferProposed {
            new_admin: new_admin.clone(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn accept_admin(env: Env) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let pending_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(ContractError::NoPendingAdmin)?;

        pending_admin.require_auth();

        let previous_admin = Self::admin(env.clone());

        env.storage()
            .instance()
            .set(&DataKey::Admin, &pending_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferred {
            previous_admin,
            new_admin: pending_admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn cancel_admin_proposal(env: Env) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        let pending_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(ContractError::NoPendingAdmin)?;

        env.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferCancelled { pending_admin }.publish(&env);

        Ok(())
    }

    fn require_initialized(env: &Env) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(())
        } else {
            Err(ContractError::NotInitialized)
        }
    }
}
