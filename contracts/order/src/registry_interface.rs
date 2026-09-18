use soroban_sdk::{contractclient, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityRecord {
    pub asset: Address,
    pub distributor: Address,
    pub buyer: Address,
    pub valid_until_ledger: u32,
}

#[contractclient(name = "RegistryClient")]
#[allow(dead_code)]
pub trait RegistryInterface {
    fn is_asset_active(env: Env, asset: Address) -> bool;

    fn is_distribution_active(env: Env, asset: Address, distributor: Address) -> bool;

    fn is_eligible(env: Env, asset: Address, distributor: Address, buyer: Address) -> bool;

    fn get_eligibility(
        env: Env,
        asset: Address,
        distributor: Address,
        buyer: Address,
    ) -> Option<EligibilityRecord>;
}
