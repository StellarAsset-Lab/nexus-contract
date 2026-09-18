# nexus-contract

Soroban smart contracts for Nexus: the on-chain layer that lets an existing
financial application discover, qualify, order, settle, verify, and
reconcile Stellar assets through a standardized integration surface.

This repository is the **contract layer only**. It does not contain a
frontend, API, indexer, worker, or SDK package.

## What the contracts do

Two contracts implement the parts of the protocol that must be enforced
on-chain:

- **Registry** (`nexus-registry`) is authoritative for which assets exist,
  which distributors are permitted to sell them, and which buyers are
  currently eligible to buy from a given distributor. Registry never holds
  customer funds.
- **Order** (`nexus-order`) is the escrow and settlement engine. It creates
  orders against a Registry-approved asset/distributor/buyer combination,
  accepts payment and asset funding from the two counterparties, and
  atomically settles, cancels, or expires the order.

## Two-contract architecture

```text
Registry
   ^
   |
Order
```

Order holds a typed client for Registry and consults it when an order is
created. Registry has no knowledge of Order and never depends on it. Once an
order is created, its eligibility window is fixed: later Registry changes
(revoking eligibility, deactivating a distribution or asset) never mutate an
existing order and never block a valid escrow from settling, being
cancelled, or expiring.

## Order lifecycle

```text
create_order -> Created
Created --fund_payment--> Created (payment_funded = true)
Created --fund_asset----> Created (asset_funded = true)
Created --settle---------> Settled     (requires both sides funded)
Created --cancel_order---> Cancelled   (requires at most one side funded)
Created --expire_order---> Expired     (after expiry, any funding state)
```

Settled, Cancelled, and Expired are terminal states. There is no path back
to Created and no admin override of any of these transitions. There is no
admin withdrawal function: every path that moves funds out of the contract
is either a settlement (to the counterparty) or a refund (back to whoever
funded that side).

## Token interface compatibility

Both contracts move funds exclusively through the standard Soroban token
interface (`soroban_sdk::token::TokenClient`, SEP-41). They accept any
token contract address that implements that interface, including Stellar
Asset Contracts. Neither contract implements a custom token.

## Repository structure

```text
nexus-contract/
├── Cargo.toml            workspace definition
├── rust-toolchain.toml   pinned toolchain
├── Makefile               build/test/lint entry points
└── contracts/
    ├── registry/          asset, distribution, and eligibility state
    └── order/              escrow and settlement engine
```

## Local development

Verify the pinned toolchain is active:

```bash
rustc --version
cargo --version
stellar --version
rustup target list --installed
```

## Test commands

```bash
make test
```

## Build commands

```bash
make build
```

This runs `stellar contract build`, which produces `wasm32v1-none` Wasm
binaries under `target/wasm32v1-none/release/`.

## Safety boundaries

These contracts are not a wallet, an exchange, an order book, a broker, a
custodian, an Anchor replacement, a KYC provider, a private-data store, a
pricing oracle, or a securities compliance engine. They do not store KYC
documents, personal data, API credentials, or off-chain analytics. All
enumeration, search, filtering, and historical queries are the
responsibility of an off-chain indexer, not the contracts.

## Deployment status

Not yet deployed. No testnet or mainnet contract IDs exist for this
repository.
