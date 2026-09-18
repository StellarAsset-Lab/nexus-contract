use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    Unauthorized = 1,
    NotInitialized = 2,
    AlreadyInitialized = 3,
    PendingAdminAlreadySet = 4,
    NoPendingAdmin = 5,
    Paused = 6,
    AssetInactive = 7,
    DistributionInactive = 8,
    BuyerNotEligible = 9,
    InvalidAmount = 10,
    InvalidExpiry = 11,
    OrderNotFound = 12,
    InvalidOrderStatus = 13,
    PaymentAlreadyFunded = 14,
    AssetAlreadyFunded = 15,
    OrderExpired = 16,
    NotExpired = 17,
    NotCancellable = 18,
    NothingToCancel = 19,
}
