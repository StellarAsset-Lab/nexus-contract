use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderStatus {
    Created,
    Settled,
    Cancelled,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderRecord {
    pub id: u64,
    pub buyer: Address,
    pub distributor: Address,
    pub asset: Address,
    pub payment_asset: Address,
    pub asset_amount: i128,
    pub payment_amount: i128,
    pub created_at_ledger: u32,
    pub expires_at_ledger: u32,
    pub status: OrderStatus,
    pub payment_funded: bool,
    pub asset_funded: bool,
}
