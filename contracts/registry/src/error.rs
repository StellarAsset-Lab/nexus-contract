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
    AssetNotFound = 6,
    AssetAlreadyActive = 7,
    IssuerMismatch = 8,
    DistributionNotFound = 9,
    DistributionAlreadyActive = 10,
    EligibilityAuthorityMismatch = 11,
    EligibilityNotFound = 12,
    InvalidEligibilityExpiry = 13,
    AssetInactive = 14,
    DistributionInactive = 15,
}
