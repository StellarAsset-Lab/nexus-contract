#![no_std]

mod error;
mod events;
mod storage;
mod types;

use error::ContractError;
use events::{AdminTransferCancelled, AdminTransferProposed, AdminTransferred};
use events::{
    AssetFunded, GatewayPaused, GatewayUnpaused, OrderCancelled, OrderCreated, OrderExpired,
    OrderSettled, PaymentFunded,
};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::DataKey;
use types::{OrderRecord, OrderStatus};

#[contract]
pub struct Order;

#[contractimpl]
impl Order {
    pub fn initialize(env: Env, admin: Address, registry: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(ContractError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Registry, &registry);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::NextOrderId, &1u64);
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

    pub fn registry(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Registry).unwrap()
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap()
    }

    pub fn next_order_id(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::NextOrderId).unwrap()
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

    pub fn pause(env: Env) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }

        env.storage().instance().set(&DataKey::Paused, &true);

        GatewayPaused {}.publish(&env);

        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let admin = Self::admin(env.clone());
        admin.require_auth();

        if !Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }

        env.storage().instance().set(&DataKey::Paused, &false);

        GatewayUnpaused {}.publish(&env);

        Ok(())
    }

    fn require_initialized(env: &Env) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(())
        } else {
            Err(ContractError::NotInitialized)
        }
    }

    fn registry_client(env: &Env) -> nexus_registry::RegistryClient<'static> {
        let registry = Self::registry(env.clone());
        nexus_registry::RegistryClient::new(env, &registry)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_order(
        env: Env,
        distributor: Address,
        buyer: Address,
        asset: Address,
        payment_asset: Address,
        asset_amount: i128,
        payment_amount: i128,
        expires_at_ledger: u32,
    ) -> Result<u64, ContractError> {
        Self::require_initialized(&env)?;

        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }

        distributor.require_auth();

        if asset_amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }
        if payment_amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }
        if expires_at_ledger <= env.ledger().sequence() {
            return Err(ContractError::InvalidExpiry);
        }

        let registry = Self::registry_client(&env);

        if !registry.is_asset_active(&asset) {
            return Err(ContractError::AssetInactive);
        }
        if !registry.is_distribution_active(&asset, &distributor) {
            return Err(ContractError::DistributionInactive);
        }
        if !registry.is_eligible(&asset, &distributor, &buyer) {
            return Err(ContractError::BuyerNotEligible);
        }

        let eligibility = registry
            .get_eligibility(&asset, &distributor, &buyer)
            .ok_or(ContractError::BuyerNotEligible)?;
        if expires_at_ledger > eligibility.valid_until_ledger {
            return Err(ContractError::BuyerNotEligible);
        }

        let order_id: u64 = env.storage().instance().get(&DataKey::NextOrderId).unwrap();
        env.storage()
            .instance()
            .set(&DataKey::NextOrderId, &(order_id + 1));

        let record = OrderRecord {
            id: order_id,
            buyer: buyer.clone(),
            distributor: distributor.clone(),
            asset: asset.clone(),
            payment_asset: payment_asset.clone(),
            asset_amount,
            payment_amount,
            created_at_ledger: env.ledger().sequence(),
            expires_at_ledger,
            status: OrderStatus::Created,
            payment_funded: false,
            asset_funded: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Order(order_id), &record);

        OrderCreated {
            order_id,
            buyer,
            distributor,
            asset,
            payment_asset,
            asset_amount,
            payment_amount,
            expires_at_ledger,
        }
        .publish(&env);

        Ok(order_id)
    }

    pub fn fund_payment(env: Env, order_id: u64) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }

        let key = DataKey::Order(order_id);
        let mut record: OrderRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::OrderNotFound)?;

        if record.status != OrderStatus::Created {
            return Err(ContractError::InvalidOrderStatus);
        }
        if env.ledger().sequence() > record.expires_at_ledger {
            return Err(ContractError::OrderExpired);
        }
        if record.payment_funded {
            return Err(ContractError::PaymentAlreadyFunded);
        }

        record.buyer.require_auth();

        let token = TokenClient::new(&env, &record.payment_asset);
        token.transfer(
            &record.buyer,
            &env.current_contract_address(),
            &record.payment_amount,
        );

        record.payment_funded = true;
        env.storage().persistent().set(&key, &record);

        PaymentFunded {
            order_id,
            buyer: record.buyer,
            payment_amount: record.payment_amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn fund_asset(env: Env, order_id: u64) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }

        let key = DataKey::Order(order_id);
        let mut record: OrderRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::OrderNotFound)?;

        if record.status != OrderStatus::Created {
            return Err(ContractError::InvalidOrderStatus);
        }
        if env.ledger().sequence() > record.expires_at_ledger {
            return Err(ContractError::OrderExpired);
        }
        if record.asset_funded {
            return Err(ContractError::AssetAlreadyFunded);
        }

        record.distributor.require_auth();

        let token = TokenClient::new(&env, &record.asset);
        token.transfer(
            &record.distributor,
            &env.current_contract_address(),
            &record.asset_amount,
        );

        record.asset_funded = true;
        env.storage().persistent().set(&key, &record);

        AssetFunded {
            order_id,
            distributor: record.distributor,
            asset_amount: record.asset_amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn settle(env: Env, order_id: u64) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let key = DataKey::Order(order_id);
        let mut record: OrderRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::OrderNotFound)?;

        if record.status != OrderStatus::Created {
            return Err(ContractError::InvalidOrderStatus);
        }
        if env.ledger().sequence() > record.expires_at_ledger {
            return Err(ContractError::OrderExpired);
        }
        if !record.payment_funded || !record.asset_funded {
            return Err(ContractError::InvalidOrderStatus);
        }

        let this_contract = env.current_contract_address();

        let payment_token = TokenClient::new(&env, &record.payment_asset);
        payment_token.transfer(&this_contract, &record.distributor, &record.payment_amount);

        let asset_token = TokenClient::new(&env, &record.asset);
        asset_token.transfer(&this_contract, &record.buyer, &record.asset_amount);

        record.status = OrderStatus::Settled;
        env.storage().persistent().set(&key, &record);

        OrderSettled {
            order_id,
            buyer: record.buyer,
            distributor: record.distributor,
            asset_amount: record.asset_amount,
            payment_amount: record.payment_amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn cancel_order(env: Env, order_id: u64) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let key = DataKey::Order(order_id);
        let mut record: OrderRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::OrderNotFound)?;

        if record.status != OrderStatus::Created {
            return Err(ContractError::InvalidOrderStatus);
        }
        if env.ledger().sequence() > record.expires_at_ledger {
            return Err(ContractError::OrderExpired);
        }

        let this_contract = env.current_contract_address();
        let cancelled_by = match (record.payment_funded, record.asset_funded) {
            (false, false) => return Err(ContractError::NothingToCancel),
            (true, true) => return Err(ContractError::NotCancellable),
            (true, false) => {
                record.buyer.require_auth();
                let token = TokenClient::new(&env, &record.payment_asset);
                token.transfer(&this_contract, &record.buyer, &record.payment_amount);
                record.buyer.clone()
            }
            (false, true) => {
                record.distributor.require_auth();
                let token = TokenClient::new(&env, &record.asset);
                token.transfer(&this_contract, &record.distributor, &record.asset_amount);
                record.distributor.clone()
            }
        };

        record.status = OrderStatus::Cancelled;
        env.storage().persistent().set(&key, &record);

        OrderCancelled {
            order_id,
            cancelled_by,
        }
        .publish(&env);

        Ok(())
    }

    pub fn expire_order(env: Env, order_id: u64) -> Result<(), ContractError> {
        Self::require_initialized(&env)?;

        let key = DataKey::Order(order_id);
        let mut record: OrderRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::OrderNotFound)?;

        if record.status != OrderStatus::Created {
            return Err(ContractError::InvalidOrderStatus);
        }
        if env.ledger().sequence() <= record.expires_at_ledger {
            return Err(ContractError::NotExpired);
        }

        let this_contract = env.current_contract_address();

        if record.payment_funded {
            let token = TokenClient::new(&env, &record.payment_asset);
            token.transfer(&this_contract, &record.buyer, &record.payment_amount);
        }
        if record.asset_funded {
            let token = TokenClient::new(&env, &record.asset);
            token.transfer(&this_contract, &record.distributor, &record.asset_amount);
        }

        record.status = OrderStatus::Expired;
        let payment_refunded = record.payment_funded;
        let asset_refunded = record.asset_funded;
        env.storage().persistent().set(&key, &record);

        OrderExpired {
            order_id,
            payment_refunded,
            asset_refunded,
        }
        .publish(&env);

        Ok(())
    }
}
