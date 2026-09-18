extern crate std;

use crate::error::ContractError;
use crate::events::{
    AdminTransferCancelled, AdminTransferProposed, AdminTransferred, AssetDeactivated,
    AssetRegistered, DistributionRegistered, DistributionRevoked, EligibilityRevoked,
    EligibilitySet,
};
use crate::{Registry, RegistryClient};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{Address, Env, Event as _};

fn setup() -> (Env, RegistryClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(Registry, ());
    let client = RegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.mock_all_auths().initialize(&admin);
    (env, client, admin)
}

fn active_asset(env: &Env, client: &RegistryClient<'static>) -> (Address, Address) {
    let asset = Address::generate(env);
    let issuer = Address::generate(env);
    client.mock_all_auths().register_asset(&asset, &issuer);
    (asset, issuer)
}

fn active_distribution(
    env: &Env,
    client: &RegistryClient<'static>,
) -> (Address, Address, Address, Address) {
    let (asset, issuer) = active_asset(env, client);
    let distributor = Address::generate(env);
    let eligibility_authority = Address::generate(env);
    client
        .mock_all_auths()
        .register_distribution(&asset, &distributor, &eligibility_authority);
    (asset, issuer, distributor, eligibility_authority)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register(Registry, ());
    let client = RegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);

    assert_eq!(client.admin(), admin);
    assert_eq!(client.pending_admin(), None);
}

#[test]
fn test_duplicate_initialization_rejection() {
    let (_env, client, admin) = setup();

    let result = client.mock_all_auths().try_initialize(&admin);
    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
#[should_panic]
fn test_unauthorized_initialization() {
    let env = Env::default();
    let contract_id = env.register(Registry, ());
    let client = RegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
}

#[test]
fn test_asset_registration() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);
    let issuer = Address::generate(&env);

    client.mock_all_auths().register_asset(&asset, &issuer);
    let events = env.events().all();

    let record = client.get_asset(&asset).unwrap();
    assert_eq!(record.asset, asset);
    assert_eq!(record.issuer, issuer);
    assert!(record.active);
    assert!(client.is_asset_active(&asset));

    assert_eq!(
        events,
        std::vec![AssetRegistered { asset, issuer }.to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_duplicate_active_asset_rejection() {
    let (env, client, _admin) = setup();
    let (asset, issuer) = active_asset(&env, &client);

    let result = client.mock_all_auths().try_register_asset(&asset, &issuer);
    assert_eq!(result, Err(Ok(ContractError::AssetAlreadyActive)));
}

#[test]
fn test_asset_deactivation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer) = active_asset(&env, &client);

    client.mock_all_auths().deactivate_asset(&asset);
    let events = env.events().all();

    assert!(!client.is_asset_active(&asset));
    assert_eq!(
        events,
        std::vec![AssetDeactivated {
            asset: asset.clone()
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_inactive_asset_reactivation_with_same_issuer() {
    let (env, client, _admin) = setup();
    let (asset, issuer) = active_asset(&env, &client);
    client.mock_all_auths().deactivate_asset(&asset);

    client.mock_all_auths().register_asset(&asset, &issuer);

    assert!(client.is_asset_active(&asset));
}

#[test]
fn test_issuer_mismatch_on_reactivation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer) = active_asset(&env, &client);
    client.mock_all_auths().deactivate_asset(&asset);

    let other_issuer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_register_asset(&asset, &other_issuer);
    assert_eq!(result, Err(Ok(ContractError::IssuerMismatch)));
}

#[test]
fn test_distribution_registration() {
    let (env, client, _admin) = setup();
    let (asset, _issuer) = active_asset(&env, &client);
    let distributor = Address::generate(&env);
    let eligibility_authority = Address::generate(&env);

    client
        .mock_all_auths()
        .register_distribution(&asset, &distributor, &eligibility_authority);
    let events = env.events().all();

    let record = client.get_distribution(&asset, &distributor).unwrap();
    assert_eq!(record.eligibility_authority, eligibility_authority);
    assert!(record.active);
    assert!(client.is_distribution_active(&asset, &distributor));

    assert_eq!(
        events,
        std::vec![DistributionRegistered {
            asset,
            distributor,
            eligibility_authority,
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_duplicate_active_distribution_rejection() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, eligibility_authority) = active_distribution(&env, &client);

    let result = client.mock_all_auths().try_register_distribution(
        &asset,
        &distributor,
        &eligibility_authority,
    );
    assert_eq!(result, Err(Ok(ContractError::DistributionAlreadyActive)));
}

#[test]
fn test_distribution_revocation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);

    client
        .mock_all_auths()
        .revoke_distribution(&asset, &distributor);
    let events = env.events().all();

    assert!(!client.is_distribution_active(&asset, &distributor));
    assert_eq!(
        events,
        std::vec![DistributionRevoked {
            asset: asset.clone(),
            distributor: distributor.clone(),
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_distribution_reactivation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, authority) = active_distribution(&env, &client);
    client
        .mock_all_auths()
        .revoke_distribution(&asset, &distributor);

    let new_authority = Address::generate(&env);
    client
        .mock_all_auths()
        .register_distribution(&asset, &distributor, &new_authority);

    let record = client.get_distribution(&asset, &distributor).unwrap();
    assert!(record.active);
    assert_eq!(record.eligibility_authority, new_authority);
    assert_ne!(record.eligibility_authority, authority);
}

#[test]
fn test_eligibility_creation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;

    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);
    let events = env.events().all();

    let record = client
        .get_eligibility(&asset, &distributor, &buyer)
        .unwrap();
    assert_eq!(record.valid_until_ledger, valid_until_ledger);
    assert!(client.is_eligible(&asset, &distributor, &buyer));

    assert_eq!(
        events,
        std::vec![EligibilitySet {
            asset,
            distributor,
            buyer,
            valid_until_ledger,
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
#[should_panic]
fn test_eligibility_authority_enforcement() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;

    // No auth mocked for the configured eligibility authority, so the
    // required `require_auth` call must panic.
    client.set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);
}

#[test]
fn test_eligibility_refresh() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let first_expiry = env.ledger().sequence() + 100;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &first_expiry);

    let second_expiry = env.ledger().sequence() + 5_000;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &second_expiry);

    let record = client
        .get_eligibility(&asset, &distributor, &buyer)
        .unwrap();
    assert_eq!(record.valid_until_ledger, second_expiry);
}

#[test]
fn test_eligibility_revocation() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    client
        .mock_all_auths()
        .revoke_eligibility(&asset, &distributor, &buyer);
    let events = env.events().all();

    assert_eq!(client.get_eligibility(&asset, &distributor, &buyer), None);
    assert!(!client.is_eligible(&asset, &distributor, &buyer));
    assert_eq!(
        events,
        std::vec![EligibilityRevoked {
            asset,
            distributor,
            buyer,
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_expired_eligibility_returning_false() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 10;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    env.ledger().set_sequence_number(valid_until_ledger + 1);

    assert!(!client.is_eligible(&asset, &distributor, &buyer));
}

#[test]
fn test_inactive_asset_returning_false_eligibility() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    client.mock_all_auths().deactivate_asset(&asset);

    assert!(!client.is_eligible(&asset, &distributor, &buyer));
}

#[test]
fn test_inactive_distribution_returning_false_eligibility() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    client
        .mock_all_auths()
        .revoke_distribution(&asset, &distributor);

    assert!(!client.is_eligible(&asset, &distributor, &buyer));
}

#[test]
fn test_missing_eligibility_returning_false() {
    let (env, client, _admin) = setup();
    let (asset, _issuer, distributor, _authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);

    assert!(!client.is_eligible(&asset, &distributor, &buyer));
    assert_eq!(client.get_eligibility(&asset, &distributor, &buyer), None);
}

#[test]
fn test_admin_proposal() {
    let (env, client, _admin) = setup();
    let new_admin = Address::generate(&env);

    client.mock_all_auths().propose_admin(&new_admin);
    let events = env.events().all();

    assert_eq!(client.pending_admin(), Some(new_admin.clone()));
    assert_eq!(
        events,
        std::vec![AdminTransferProposed { new_admin }.to_xdr(&env, &client.address),],
    );
}

#[test]
#[should_panic]
fn test_unauthorized_admin_proposal() {
    let (env, client, _admin) = setup();
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);
}

#[test]
fn test_admin_acceptance() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    client.mock_all_auths().propose_admin(&new_admin);

    client.mock_all_auths().accept_admin();
    let events = env.events().all();

    assert_eq!(client.admin(), new_admin.clone());
    assert_eq!(client.pending_admin(), None);
    assert_eq!(
        events,
        std::vec![AdminTransferred {
            previous_admin: admin,
            new_admin,
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
#[should_panic]
fn test_wrong_address_failing_acceptance() {
    let (env, client, _admin) = setup();
    let new_admin = Address::generate(&env);
    client.mock_all_auths().propose_admin(&new_admin);

    // No auth mocked for the pending admin, so acceptance must fail.
    client.accept_admin();
}

#[test]
fn test_admin_proposal_cancellation() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    client.mock_all_auths().propose_admin(&new_admin);

    client.mock_all_auths().cancel_admin_proposal();
    let events = env.events().all();

    assert_eq!(client.pending_admin(), None);
    assert_eq!(client.admin(), admin);
    assert_eq!(
        events,
        std::vec![AdminTransferCancelled {
            pending_admin: new_admin,
        }
        .to_xdr(&env, &client.address),],
    );
}

#[test]
fn test_read_apis() {
    let (env, client, admin) = setup();
    let (asset, issuer, distributor, authority) = active_distribution(&env, &client);
    let buyer = Address::generate(&env);
    let valid_until_ledger = env.ledger().sequence() + 1_000;
    client
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    assert_eq!(client.admin(), admin);
    assert_eq!(client.pending_admin(), None);

    let asset_record = client.get_asset(&asset).unwrap();
    assert_eq!(asset_record.issuer, issuer);
    assert!(client.is_asset_active(&asset));

    let distribution_record = client.get_distribution(&asset, &distributor).unwrap();
    assert_eq!(distribution_record.eligibility_authority, authority);
    assert!(client.is_distribution_active(&asset, &distributor));

    let eligibility_record = client
        .get_eligibility(&asset, &distributor, &buyer)
        .unwrap();
    assert_eq!(eligibility_record.valid_until_ledger, valid_until_ledger);
    assert!(client.is_eligible(&asset, &distributor, &buyer));

    let missing_asset = Address::generate(&env);
    assert_eq!(client.get_asset(&missing_asset), None);
    assert!(!client.is_asset_active(&missing_asset));
}
