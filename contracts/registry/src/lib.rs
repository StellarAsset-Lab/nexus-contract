#![no_std]

mod error;
mod events;
mod storage;
mod types;

use error::ContractError;
use events::{AdminTransferCancelled, AdminTransferProposed, AdminTransferred};
use events::{AssetDeactivated, AssetRegistered};
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::DataKey;
use types::AssetRecord;

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

    pub fn register_asset(env: Env, asset: Address, issuer: Address) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        let key = DataKey::Asset(asset.clone());
        let existing: Option<AssetRecord> = env.storage().persistent().get(&key);

        match existing {
            Some(record) if record.active => return Err(ContractError::AssetAlreadyActive),
            Some(record) if record.issuer != issuer => return Err(ContractError::IssuerMismatch),
            _ => {}
        }

        let record = AssetRecord {
            asset: asset.clone(),
            issuer: issuer.clone(),
            active: true,
        };
        env.storage().persistent().set(&key, &record);

        AssetRegistered { asset, issuer }.publish(&env);

        Ok(())
    }

    pub fn deactivate_asset(env: Env, asset: Address) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        let key = DataKey::Asset(asset.clone());
        let mut record: AssetRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::AssetNotFound)?;

        if !record.active {
            return Err(ContractError::AssetInactive);
        }

        record.active = false;
        env.storage().persistent().set(&key, &record);

        AssetDeactivated { asset }.publish(&env);

        Ok(())
    }

    pub fn get_asset(env: Env, asset: Address) -> Option<AssetRecord> {
        env.storage().persistent().get(&DataKey::Asset(asset))
    }

    pub fn is_asset_active(env: Env, asset: Address) -> bool {
        Self::get_asset(env, asset)
            .map(|record| record.active)
            .unwrap_or(false)
    }

    fn require_initialized(env: &Env) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(())
        } else {
            Err(ContractError::NotInitialized)
        }
    }
}
