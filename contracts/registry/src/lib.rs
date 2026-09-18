#![no_std]

mod error;
mod events;
mod storage;
mod types;

use error::ContractError;
use events::{AdminTransferCancelled, AdminTransferProposed, AdminTransferred};
use events::{AssetDeactivated, AssetRegistered};
use events::{DistributionRegistered, DistributionRevoked};
use events::{EligibilityRevoked, EligibilitySet};
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::{extend_instance_ttl, extend_persistent_ttl, DataKey};
use types::{AssetRecord, DistributionConfig, EligibilityRecord};

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
        extend_instance_ttl(&env);

        Ok(())
    }

    pub fn admin(env: Env) -> Address {
        extend_instance_ttl(&env);
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn pending_admin(env: Env) -> Option<Address> {
        extend_instance_ttl(&env);
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
        extend_persistent_ttl(&env, &key);

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
        extend_persistent_ttl(&env, &key);

        AssetDeactivated { asset }.publish(&env);

        Ok(())
    }

    pub fn get_asset(env: Env, asset: Address) -> Option<AssetRecord> {
        let key = DataKey::Asset(asset);
        let record: Option<AssetRecord> = env.storage().persistent().get(&key);
        if record.is_some() {
            extend_persistent_ttl(&env, &key);
        }
        record
    }

    pub fn is_asset_active(env: Env, asset: Address) -> bool {
        Self::get_asset(env, asset)
            .map(|record| record.active)
            .unwrap_or(false)
    }

    pub fn register_distribution(
        env: Env,
        asset: Address,
        distributor: Address,
        eligibility_authority: Address,
    ) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        if !Self::is_asset_active(env.clone(), asset.clone()) {
            return Err(ContractError::AssetInactive);
        }

        let key = DataKey::Distribution(asset.clone(), distributor.clone());
        let existing: Option<DistributionConfig> = env.storage().persistent().get(&key);

        if let Some(record) = &existing {
            if record.active {
                return Err(ContractError::DistributionAlreadyActive);
            }
        }

        let record = DistributionConfig {
            asset: asset.clone(),
            distributor: distributor.clone(),
            eligibility_authority: eligibility_authority.clone(),
            active: true,
        };
        env.storage().persistent().set(&key, &record);
        extend_persistent_ttl(&env, &key);

        DistributionRegistered {
            asset,
            distributor,
            eligibility_authority,
        }
        .publish(&env);

        Ok(())
    }

    pub fn revoke_distribution(
        env: Env,
        asset: Address,
        distributor: Address,
    ) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        let key = DataKey::Distribution(asset.clone(), distributor.clone());
        let mut record: DistributionConfig = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::DistributionNotFound)?;

        if !record.active {
            return Err(ContractError::DistributionInactive);
        }

        record.active = false;
        env.storage().persistent().set(&key, &record);
        extend_persistent_ttl(&env, &key);

        DistributionRevoked { asset, distributor }.publish(&env);

        Ok(())
    }

    pub fn get_distribution(
        env: Env,
        asset: Address,
        distributor: Address,
    ) -> Option<DistributionConfig> {
        let key = DataKey::Distribution(asset, distributor);
        let record: Option<DistributionConfig> = env.storage().persistent().get(&key);
        if record.is_some() {
            extend_persistent_ttl(&env, &key);
        }
        record
    }

    pub fn is_distribution_active(env: Env, asset: Address, distributor: Address) -> bool {
        Self::get_distribution(env, asset, distributor)
            .map(|record| record.active)
            .unwrap_or(false)
    }

    pub fn set_eligibility(
        env: Env,
        asset: Address,
        distributor: Address,
        buyer: Address,
        valid_until_ledger: u32,
    ) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        if !Self::is_asset_active(env.clone(), asset.clone()) {
            return Err(ContractError::AssetInactive);
        }

        let distribution: DistributionConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Distribution(asset.clone(), distributor.clone()))
            .ok_or(ContractError::DistributionNotFound)?;

        if !distribution.active {
            return Err(ContractError::DistributionInactive);
        }

        distribution.eligibility_authority.require_auth();

        if valid_until_ledger < env.ledger().sequence() {
            return Err(ContractError::InvalidEligibilityExpiry);
        }

        let key = DataKey::Eligibility(asset.clone(), distributor.clone(), buyer.clone());
        let record = EligibilityRecord {
            asset: asset.clone(),
            distributor: distributor.clone(),
            buyer: buyer.clone(),
            valid_until_ledger,
        };
        env.storage().persistent().set(&key, &record);
        extend_persistent_ttl(&env, &key);

        EligibilitySet {
            asset,
            distributor,
            buyer,
            valid_until_ledger,
        }
        .publish(&env);

        Ok(())
    }

    pub fn revoke_eligibility(
        env: Env,
        asset: Address,
        distributor: Address,
        buyer: Address,
    ) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let distribution: DistributionConfig = env
            .storage()
            .persistent()
            .get(&DataKey::Distribution(asset.clone(), distributor.clone()))
            .ok_or(ContractError::DistributionNotFound)?;

        distribution.eligibility_authority.require_auth();

        let key = DataKey::Eligibility(asset.clone(), distributor.clone(), buyer.clone());
        if !env.storage().persistent().has(&key) {
            return Err(ContractError::EligibilityNotFound);
        }
        env.storage().persistent().remove(&key);

        EligibilityRevoked {
            asset,
            distributor,
            buyer,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_eligibility(
        env: Env,
        asset: Address,
        distributor: Address,
        buyer: Address,
    ) -> Option<EligibilityRecord> {
        let key = DataKey::Eligibility(asset, distributor, buyer);
        let record: Option<EligibilityRecord> = env.storage().persistent().get(&key);
        if record.is_some() {
            extend_persistent_ttl(&env, &key);
        }
        record
    }

    pub fn is_eligible(env: Env, asset: Address, distributor: Address, buyer: Address) -> bool {
        if !Self::is_asset_active(env.clone(), asset.clone()) {
            return false;
        }
        if !Self::is_distribution_active(env.clone(), asset.clone(), distributor.clone()) {
            return false;
        }
        match Self::get_eligibility(env.clone(), asset, distributor, buyer) {
            Some(record) => env.ledger().sequence() <= record.valid_until_ledger,
            None => false,
        }
    }

    fn require_initialized(env: &Env) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(())
        } else {
            Err(ContractError::NotInitialized)
        }
    }
}
