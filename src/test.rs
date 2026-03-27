#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// Helper struct to boilerplate the setup
struct TestEnv {
    env: Env,
    admin: Address,
    beneficiary: Address,
    token_admin: Address,
    token: token::Client<'static>,
    token_asset: token::StellarAssetClient<'static>,
    contract_id: Address,
    client: VestingContractClient<'static>,
}

impl TestEnv {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths(); // Automatically handles require_auth() calls

        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);

        // Deploy the Token (Stellar Asset Contract)
        let token_id = env.register_stellar_asset_contract(token_admin.clone());
        let token = token::Client::new(&env, &token_id);
        let token_asset = token::StellarAssetClient::new(&env, &token_id);

        // Deploy the Vesting Contract
        let contract_id = env.register_contract(None, VestingContract);
        let client = VestingContractClient::new(&env, &contract_id);

        // Give the admin some tokens to start with
        token_asset.mint(&admin, &1_000_000);

        TestEnv {
            env,
            admin,
            beneficiary,
            token_admin,
            token,
            token_asset,
            contract_id,
            client,
        }
    }
}

#[test]
fn test_initialize() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    // Verify it cannot be initialized twice
    let result = t.client.try_initialize(&t.admin, &t.token.address);
    assert_eq!(result.err(), Some(Ok(VestingError::AlreadyInitialized)));
}

#[test]
fn test_create_vesting_transfers_tokens() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    let amount = 100_000i128;
    t.client.create_vesting(&t.beneficiary, &amount, &0, &0, &1000);

    // Tokens should move from admin to the contract
    assert_eq!(t.token.balance(&t.admin), 900_000);
    assert_eq!(t.token.balance(&t.contract_id), 100_000);
}

#[test]
fn test_vesting_cliff_enforcement() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    let total = 1000i128;
    let start = 100u64;
    let cliff = 500u64;
    let duration = 1000u64; // Ends at 1100

    t.client.create_vesting(&t.beneficiary, &total, &start, &cliff, &duration);

    // 1. Before Cliff: Should be 0 claimable
    t.env.ledger().set_timestamp(499);
    assert_eq!(t.client.get_claimable_amount(&t.beneficiary, &0), 0);
    assert_eq!(t.client.claim(&t.beneficiary, &0), 0);

    // 2. At Cliff: Progress = (500 - 100) / 1000 = 40%
    // 40% of 1000 = 400 tokens
    t.env.ledger().set_timestamp(500);
    assert_eq!(t.client.get_claimable_amount(&t.beneficiary, &0), 400);
    
    t.client.claim(&t.beneficiary, &0);
    assert_eq!(t.token.balance(&t.beneficiary), 400);
}

#[test]
fn test_incremental_claiming() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    // 1000 tokens over 1000 seconds, no cliff
    t.client.create_vesting(&t.beneficiary, &1000, &0, &0, &1000);

    // At 250s, claim 250
    t.env.ledger().set_timestamp(250);
    t.client.claim(&t.beneficiary, &0);
    assert_eq!(t.token.balance(&t.beneficiary), 250);

    // At 500s, get_claimable should show 250 (500 total - 250 already claimed)
    t.env.ledger().set_timestamp(500);
    assert_eq!(t.client.get_claimable_amount(&t.beneficiary, &0), 250);
    t.client.claim(&t.beneficiary, &0);
    assert_eq!(t.token.balance(&t.beneficiary), 500);

    // At 1500s (past end), claim remaining 500
    t.env.ledger().set_timestamp(1500);
    t.client.claim(&t.beneficiary, &0);
    assert_eq!(t.token.balance(&t.beneficiary), 1000);
}

#[test]
fn test_emergency_withdraw() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    // Mint tokens directly to contract to simulate a pool
    t.token_asset.mint(&t.contract_id, &50_000);
    
    let admin_before = t.token.balance(&t.admin);
    t.client.emergency_withdraw(&50_000);

    assert_eq!(t.token.balance(&t.contract_id), 0);
    assert_eq!(t.token.balance(&t.admin), admin_before + 50_000);
}

#[test]
fn test_invalid_params() {
    let t = TestEnv::setup();
    t.client.initialize(&t.admin, &t.token.address);

    // Duration 0 should fail
    let res = t.client.try_create_vesting(&t.beneficiary, &100, &0, &0, &0);
    assert_eq!(res.err(), Some(Ok(VestingError::InvalidParam)));

    // Cliff before start should fail
    let res = t.client.try_create_vesting(&t.beneficiary, &100, &1000, &500, &1000);
    assert_eq!(res.err(), Some(Ok(VestingError::InvalidParam)));
}