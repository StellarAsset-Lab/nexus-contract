extern crate std;

use crate::error::ContractError;
use crate::events::{AssetFunded, OrderCreated, PaymentFunded};
use crate::{Order, OrderClient};
use nexus_registry::{Registry, RegistryClient};
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env, Event as _};

struct Fixture {
    env: Env,
    registry: RegistryClient<'static>,
    order: OrderClient<'static>,
    distributor: Address,
    buyer: Address,
    asset: Address,
    payment_asset: Address,
}

const VALID_LEDGERS: u32 = 100_000;

fn setup() -> Fixture {
    let env = Env::default();

    let registry_admin = Address::generate(&env);
    let registry_id = env.register(Registry, ());
    let registry = RegistryClient::new(&env, &registry_id);
    registry.mock_all_auths().initialize(&registry_admin);

    let order_admin = Address::generate(&env);
    let order_id = env.register(Order, ());
    let order = OrderClient::new(&env, &order_id);
    order
        .mock_all_auths()
        .initialize(&order_admin, &registry_id);

    let asset_issuer = Address::generate(&env);
    let distribution_sac = env.register_stellar_asset_contract_v2(asset_issuer.clone());
    let asset = distribution_sac.address();

    let payment_issuer = Address::generate(&env);
    let payment_sac = env.register_stellar_asset_contract_v2(payment_issuer.clone());
    let payment_asset = payment_sac.address();

    let distributor = Address::generate(&env);
    let buyer = Address::generate(&env);
    let eligibility_authority = Address::generate(&env);

    registry
        .mock_all_auths()
        .register_asset(&asset, &asset_issuer);
    registry
        .mock_all_auths()
        .register_distribution(&asset, &distributor, &eligibility_authority);
    let valid_until_ledger = env.ledger().sequence() + VALID_LEDGERS;
    registry
        .mock_all_auths()
        .set_eligibility(&asset, &distributor, &buyer, &valid_until_ledger);

    StellarAssetClient::new(&env, &asset)
        .mock_all_auths()
        .mint(&distributor, &1_000_000_000);
    StellarAssetClient::new(&env, &payment_asset)
        .mock_all_auths()
        .mint(&buyer, &1_000_000_000);

    Fixture {
        env,
        registry,
        order,
        distributor,
        buyer,
        asset,
        payment_asset,
    }
}

impl Fixture {
    fn create_order(&self, asset_amount: i128, payment_amount: i128, expiry_offset: u32) -> u64 {
        let expires_at_ledger = self.env.ledger().sequence() + expiry_offset;
        self.order.mock_all_auths().create_order(
            &self.distributor,
            &self.buyer,
            &self.asset,
            &self.payment_asset,
            &asset_amount,
            &payment_amount,
            &expires_at_ledger,
        )
    }
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let registry_admin = Address::generate(&env);
    let registry_id = env.register(Registry, ());
    let registry = RegistryClient::new(&env, &registry_id);
    registry.mock_all_auths().initialize(&registry_admin);

    let order_admin = Address::generate(&env);
    let order_id = env.register(Order, ());
    let order = OrderClient::new(&env, &order_id);
    order
        .mock_all_auths()
        .initialize(&order_admin, &registry_id);

    assert_eq!(order.admin(), order_admin);
    assert_eq!(order.registry(), registry_id);
    assert!(!order.is_paused());
    assert_eq!(order.next_order_id(), 1);
}

#[test]
fn test_duplicate_initialization_rejection() {
    let fixture = setup();
    let registry_id = fixture.registry.address.clone();
    let admin = fixture.order.admin();

    let result = fixture
        .order
        .mock_all_auths()
        .try_initialize(&admin, &registry_id);
    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
fn test_admin_transfer() {
    let fixture = setup();
    let new_admin = Address::generate(&fixture.env);

    fixture.order.mock_all_auths().propose_admin(&new_admin);
    assert_eq!(fixture.order.pending_admin(), Some(new_admin.clone()));

    fixture.order.mock_all_auths().accept_admin();
    assert_eq!(fixture.order.admin(), new_admin);
    assert_eq!(fixture.order.pending_admin(), None);
}

#[test]
fn test_pause_and_unpause() {
    let fixture = setup();

    fixture.order.mock_all_auths().pause();
    assert!(fixture.order.is_paused());

    fixture.order.mock_all_auths().unpause();
    assert!(!fixture.order.is_paused());
}

#[test]
fn test_paused_create_rejection() {
    let fixture = setup();
    fixture.order.mock_all_auths().pause();

    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;
    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::Paused)));
}

#[test]
fn test_paused_funding_rejection() {
    let fixture = setup();
    let order_id = fixture.create_order(100, 100, 1_000);
    fixture.order.mock_all_auths().pause();

    let payment_result = fixture.order.mock_all_auths().try_fund_payment(&order_id);
    assert_eq!(payment_result, Err(Ok(ContractError::Paused)));

    let asset_result = fixture.order.mock_all_auths().try_fund_asset(&order_id);
    assert_eq!(asset_result, Err(Ok(ContractError::Paused)));
}

#[test]
fn test_inactive_asset_create_rejection() {
    let fixture = setup();
    fixture
        .registry
        .mock_all_auths()
        .deactivate_asset(&fixture.asset);

    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;
    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::AssetInactive)));
}

#[test]
fn test_inactive_distribution_create_rejection() {
    let fixture = setup();
    fixture
        .registry
        .mock_all_auths()
        .revoke_distribution(&fixture.asset, &fixture.distributor);

    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;
    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::DistributionInactive)));
}

#[test]
fn test_ineligible_buyer_create_rejection() {
    let fixture = setup();
    let stranger = Address::generate(&fixture.env);

    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;
    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &stranger,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::BuyerNotEligible)));
}

#[test]
fn test_expiry_beyond_eligibility_validity_rejection() {
    let fixture = setup();

    let expires_at_ledger = fixture.env.ledger().sequence() + VALID_LEDGERS + 1;
    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::BuyerNotEligible)));
}

#[test]
fn test_zero_amount_rejection() {
    let fixture = setup();
    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;

    let zero_asset = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &0i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(zero_asset, Err(Ok(ContractError::InvalidAmount)));

    let zero_payment = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &0i128,
        &expires_at_ledger,
    );
    assert_eq!(zero_payment, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_already_expired_create_rejection() {
    let fixture = setup();
    let expires_at_ledger = fixture.env.ledger().sequence();

    let result = fixture.order.mock_all_auths().try_create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &100i128,
        &100i128,
        &expires_at_ledger,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidExpiry)));
}

#[test]
fn test_successful_order_creation() {
    let fixture = setup();
    let expires_at_ledger = fixture.env.ledger().sequence() + 1_000;

    let order_id = fixture.order.mock_all_auths().create_order(
        &fixture.distributor,
        &fixture.buyer,
        &fixture.asset,
        &fixture.payment_asset,
        &500i128,
        &1000i128,
        &expires_at_ledger,
    );
    let events = fixture.env.events().all();

    assert_eq!(order_id, 1);
    let record = fixture.order.get_order(&order_id).unwrap();
    assert_eq!(record.asset_amount, 500);
    assert_eq!(record.payment_amount, 1000);
    assert!(!record.payment_funded);
    assert!(!record.asset_funded);

    assert_eq!(
        events,
        std::vec![OrderCreated {
            order_id,
            buyer: fixture.buyer.clone(),
            distributor: fixture.distributor.clone(),
            asset: fixture.asset.clone(),
            payment_asset: fixture.payment_asset.clone(),
            asset_amount: 500,
            payment_amount: 1000,
            expires_at_ledger,
        }
        .to_xdr(&fixture.env, &fixture.order.address),],
    );
}

#[test]
fn test_sequential_order_ids() {
    let fixture = setup();

    let first = fixture.create_order(100, 100, 1_000);
    let second = fixture.create_order(100, 100, 1_000);
    let third = fixture.create_order(100, 100, 1_000);

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(third, 3);
}

#[test]
fn test_payment_funding() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);

    fixture.order.mock_all_auths().fund_payment(&order_id);
    let events = fixture
        .env
        .events()
        .all()
        .filter_by_contract(&fixture.order.address);

    let record = fixture.order.get_order(&order_id).unwrap();
    assert!(record.payment_funded);

    let token = TokenClient::new(&fixture.env, &fixture.payment_asset);
    assert_eq!(token.balance(&fixture.order.address), 1000);

    assert_eq!(
        events,
        std::vec![PaymentFunded {
            order_id,
            buyer: fixture.buyer.clone(),
            payment_amount: 1000,
        }
        .to_xdr(&fixture.env, &fixture.order.address),],
    );
}

#[test]
#[should_panic]
fn test_payment_funding_authorization() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);

    // No auth mocked for the buyer, so the required `require_auth` call
    // inside `fund_payment` must panic.
    fixture.order.fund_payment(&order_id);
}

#[test]
fn test_duplicate_payment_funding() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);
    fixture.order.mock_all_auths().fund_payment(&order_id);

    let result = fixture.order.mock_all_auths().try_fund_payment(&order_id);
    assert_eq!(result, Err(Ok(ContractError::PaymentAlreadyFunded)));
}

#[test]
fn test_asset_funding() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);

    fixture.order.mock_all_auths().fund_asset(&order_id);
    let events = fixture
        .env
        .events()
        .all()
        .filter_by_contract(&fixture.order.address);

    let record = fixture.order.get_order(&order_id).unwrap();
    assert!(record.asset_funded);

    let token = TokenClient::new(&fixture.env, &fixture.asset);
    assert_eq!(token.balance(&fixture.order.address), 500);

    assert_eq!(
        events,
        std::vec![AssetFunded {
            order_id,
            distributor: fixture.distributor.clone(),
            asset_amount: 500,
        }
        .to_xdr(&fixture.env, &fixture.order.address),],
    );
}

#[test]
#[should_panic]
fn test_asset_funding_authorization() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);

    // No auth mocked for the distributor, so the required `require_auth`
    // call inside `fund_asset` must panic.
    fixture.order.fund_asset(&order_id);
}

#[test]
fn test_duplicate_asset_funding() {
    let fixture = setup();
    let order_id = fixture.create_order(500, 1000, 1_000);
    fixture.order.mock_all_auths().fund_asset(&order_id);

    let result = fixture.order.mock_all_auths().try_fund_asset(&order_id);
    assert_eq!(result, Err(Ok(ContractError::AssetAlreadyFunded)));
}
