use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRecord {
    pub asset: Address,
    pub issuer: Address,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionConfig {
    pub asset: Address,
    pub distributor: Address,
    pub eligibility_authority: Address,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityRecord {
    pub asset: Address,
    pub distributor: Address,
    pub buyer: Address,
    pub valid_until_ledger: u32,
}
