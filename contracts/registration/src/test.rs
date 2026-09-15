#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::Map;

// Helper: spins up a test token contract and returns clients for
// normal token operations (transfer/balance) and admin operations (mint).
fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let address = sac.address();
    (
        TokenClient::new(env, &address),
        StellarAssetClient::new(env, &address),
    )
}

fn create_token_prices(env: &Env, token: &Address, price: i128) -> Map<Address, i128> {
    let mut map = Map::new(env);
    map.set(token.clone(), price);
    map
}

#[test]
fn test_free_event_register_and_checkin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _) = create_token_contract(&env, &token_admin);

    let prices = create_token_prices(&env, &token.address, 0);

    client.create_event(&organizer, &1, &prices, &100, &false, &0);
    client.register(&attendee, &1, &token.address);

    client.check_in(&organizer, &1, &attendee);

    let checked_in: bool = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&DataKey::CheckedIn(1, attendee.clone()))
            .unwrap()
    });
    assert!(checked_in);
}

#[test]
fn test_paid_event_register_transfers_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &0);
    client.register(&attendee, &1, &token.address);

    assert_eq!(token.balance(&attendee), 800);
    assert_eq!(token.balance(&contract_id), 200);
    assert_eq!(token.balance(&organizer), 0);
}

#[test]
fn test_payout_moves_escrowed_funds_to_organizer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &0);
    client.register(&attendee, &1, &token.address);
    assert_eq!(token.balance(&contract_id), 200);

    client.payout(&organizer, &1);

    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(token.balance(&organizer), 200);
}

#[test]
fn test_self_refund_before_deadline_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &9_999_999_999);
    client.register(&attendee, &1, &token.address);
    assert_eq!(token.balance(&attendee), 800);

    client.refund(&attendee, &1, &attendee);
    assert_eq!(token.balance(&attendee), 1000);
}

#[test]
#[should_panic(expected = "refund deadline passed")]
fn test_self_refund_after_deadline_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &100);
    client.register(&attendee, &1, &token.address);

    env.ledger().with_mut(|li| li.timestamp = 200);

    client.refund(&attendee, &1, &attendee); // should panic
}

#[test]
fn test_organizer_refund_bypasses_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &false, &100);
    client.register(&attendee, &1, &token.address);
    env.ledger().with_mut(|li| li.timestamp = 500);

    client.refund(&organizer, &1, &attendee);
    assert_eq!(token.balance(&attendee), 1000);
}

#[test]
#[should_panic(expected = "only attendee or organizer can refund")]
fn test_stranger_cannot_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let stranger = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    token_admin_client.mint(&attendee, &1000);

    let prices = create_token_prices(&env, &token.address, 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &0);
    client.register(&attendee, &1, &token.address);

    client.refund(&stranger, &1, &attendee); // should panic
}

#[test]
fn test_multiple_tokens() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee_a = Address::generate(&env);
    let attendee_b = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_a, token_a_admin) = create_token_contract(&env, &token_admin);
    let (token_b, token_b_admin) = create_token_contract(&env, &token_admin);

    token_a_admin.mint(&attendee_a, &1000);
    token_b_admin.mint(&attendee_b, &1000);

    let mut prices = Map::new(&env);
    prices.set(token_a.address.clone(), 100);
    prices.set(token_b.address.clone(), 200);

    client.create_event(&organizer, &1, &prices, &100, &true, &0);

    client.register(&attendee_a, &1, &token_a.address);
    client.register(&attendee_b, &1, &token_b.address);

    assert_eq!(token_a.balance(&contract_id), 100);
    assert_eq!(token_b.balance(&contract_id), 200);

    // payout
    client.payout(&organizer, &1);
    assert_eq!(token_a.balance(&organizer), 100);
    assert_eq!(token_b.balance(&organizer), 200);
}

#[test]
#[should_panic(expected = "unsupported token")]
fn test_unsupported_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_a, _) = create_token_contract(&env, &token_admin);
    let (token_b, _) = create_token_contract(&env, &token_admin);

    let prices = create_token_prices(&env, &token_a.address, 100);

    client.create_event(&organizer, &1, &prices, &100, &true, &0);

    // Attempting to register with token_b, which is not in prices Map
    client.register(&attendee, &1, &token_b.address);
}
