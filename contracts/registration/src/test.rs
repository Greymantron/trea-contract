#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::{StellarAssetClient, TokenClient};

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

    client.create_event(&organizer, &1, &0, &token.address, &100, &false, &0);
    client.register(&attendee, &1);

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

    client.create_event(&organizer, &1, &200, &token.address, &100, &true, &0);
    client.register(&attendee, &1);

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

    client.create_event(&organizer, &1, &200, &token.address, &100, &true, &0);
    client.register(&attendee, &1);
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

    client.create_event(
        &organizer,
        &1,
        &200,
        &token.address,
        &100,
        &true,
        &9_999_999_999,
    );
    client.register(&attendee, &1);
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

    client.create_event(&organizer, &1, &200, &token.address, &100, &true, &100);
    client.register(&attendee, &1);

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

    client.create_event(&organizer, &1, &200, &token.address, &100, &false, &100);
    client.register(&attendee, &1);
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

    client.create_event(&organizer, &1, &200, &token.address, &100, &true, &0);
    client.register(&attendee, &1);

    client.refund(&stranger, &1, &attendee); // should panic
}

#[test]
fn test_update_event_terms_before_registration_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    client.create_event(&organizer, &1, &200, &token.address, &100, &false, &0);
    client.update_event_terms(&organizer, &1, &300, &true, &1000);

    let attendee = Address::generate(&env);
    token_admin_client.mint(&attendee, &1000);

    client.register(&attendee, &1);

    assert_eq!(token.balance(&attendee), 700);
    assert_eq!(token.balance(&contract_id), 300);

    client.refund(&attendee, &1, &attendee);
    assert_eq!(token.balance(&attendee), 1000);
}

#[test]
#[should_panic(expected = "event already has registrations")]
fn test_update_event_terms_after_registration_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let attendee = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, token_admin_client) = create_token_contract(&env, &token_admin);

    client.create_event(&organizer, &1, &200, &token.address, &100, &false, &0);

    token_admin_client.mint(&attendee, &1000);
    client.register(&attendee, &1);

    client.update_event_terms(&organizer, &1, &300, &true, &1000);
}

#[test]
#[should_panic(expected = "not the organizer")]
fn test_update_event_terms_not_organizer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EventRegistration, ());
    let client = EventRegistrationClient::new(&env, &contract_id);

    let organizer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _) = create_token_contract(&env, &token_admin);

    client.create_event(&organizer, &1, &200, &token.address, &100, &false, &0);
    client.update_event_terms(&stranger, &1, &300, &true, &1000);
}
