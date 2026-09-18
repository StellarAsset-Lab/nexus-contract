use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCreated {
    #[topic]
    pub order_id: u64,
    #[topic]
    pub buyer: Address,
    #[topic]
    pub distributor: Address,
    pub asset: Address,
    pub payment_asset: Address,
    pub asset_amount: i128,
    pub payment_amount: i128,
    pub expires_at_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentFunded {
    #[topic]
    pub order_id: u64,
    #[topic]
    pub buyer: Address,
    pub payment_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetFunded {
    #[topic]
    pub order_id: u64,
    #[topic]
    pub distributor: Address,
    pub asset_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderSettled {
    #[topic]
    pub order_id: u64,
    #[topic]
    pub buyer: Address,
    #[topic]
    pub distributor: Address,
    pub asset_amount: i128,
    pub payment_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCancelled {
    #[topic]
    pub order_id: u64,
    #[topic]
    pub cancelled_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderExpired {
    #[topic]
    pub order_id: u64,
    pub payment_refunded: bool,
    pub asset_refunded: bool,
}

#[contractevent]
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct GatewayPaused {}

#[contractevent]
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct GatewayUnpaused {}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferProposed {
    #[topic]
    pub new_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferred {
    #[topic]
    pub previous_admin: Address,
    pub new_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferCancelled {
    #[topic]
    pub pending_admin: Address,
}
