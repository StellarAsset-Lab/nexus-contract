use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRegistered {
    #[topic]
    pub asset: Address,
    pub issuer: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetDeactivated {
    #[topic]
    pub asset: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionRegistered {
    #[topic]
    pub asset: Address,
    #[topic]
    pub distributor: Address,
    pub eligibility_authority: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionRevoked {
    #[topic]
    pub asset: Address,
    #[topic]
    pub distributor: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilitySet {
    #[topic]
    pub asset: Address,
    #[topic]
    pub distributor: Address,
    #[topic]
    pub buyer: Address,
    pub valid_until_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityRevoked {
    #[topic]
    pub asset: Address,
    #[topic]
    pub distributor: Address,
    #[topic]
    pub buyer: Address,
}

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
