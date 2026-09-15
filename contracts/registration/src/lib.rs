#![no_std]
#![allow(clippy::too_many_arguments)]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct Event {
    pub organizer: Address,
    pub price: i128,
    pub token: Address,
    pub capacity: u32,
    pub registered: u32,
    pub self_refund_allowed: bool,
    pub refund_deadline: u64,
}

#[contracttype]
pub enum DataKey {
    Event(u32),
    Registered(u32, Address),
    CheckedIn(u32, Address),
}

#[contract]
pub struct EventRegistration;

#[contractimpl]
impl EventRegistration {
    pub fn create_event(
        env: Env,
        organizer: Address,
        event_id: u32,
        price: i128,
        token: Address,
        capacity: u32,
        self_refund_allowed: bool,
        refund_deadline: u64,
    ) {
        organizer.require_auth();
        let event = Event {
            organizer,
            price,
            token,
            capacity,
            registered: 0,
            self_refund_allowed,
            refund_deadline,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
    }

    pub fn update_event_terms(
        env: Env,
        organizer: Address,
        event_id: u32,
        price: i128,
        self_refund_allowed: bool,
        refund_deadline: u64,
    ) {
        organizer.require_auth();
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap();
        assert!(caller_is_organizer(&event, &organizer), "not the organizer");
        assert!(event.registered == 0, "event already has registrations");

        event.price = price;
        event.self_refund_allowed = self_refund_allowed;
        event.refund_deadline = refund_deadline;

        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
    }

    pub fn check_in(env: Env, organizer: Address, event_id: u32, attendee: Address) {
        organizer.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::CheckedIn(event_id, attendee), &true);
    }

    pub fn register(env: Env, attendee: Address, event_id: u32) {
        attendee.require_auth();
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap();
        assert!(event.registered < event.capacity, "event full");

        if event.price > 0 {
            let client = token::Client::new(&env, &event.token);
            // pay INTO the contract's own balance, not the organizer, so refunds
            // don't require the organizer's live signature later.
            let contract_addr = env.current_contract_address();
            client.transfer(&attendee, &contract_addr, &event.price);
        }

        event.registered += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        env.storage()
            .persistent()
            .set(&DataKey::Registered(event_id, attendee), &event.price);
    }

    pub fn refund(env: Env, caller: Address, event_id: u32, attendee: Address) {
        caller.require_auth();

        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap();
        let paid: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Registered(event_id, attendee.clone()))
            .expect("not registered");

        let is_organizer = caller == event.organizer;
        let is_self = caller == attendee;

        if is_self {
            assert!(
                event.self_refund_allowed,
                "self-refund not allowed for this event"
            );
            if event.refund_deadline > 0 {
                assert!(
                    env.ledger().timestamp() < event.refund_deadline,
                    "refund deadline passed"
                );
            }
        } else {
            assert!(is_organizer, "only attendee or organizer can refund");
        }

        if paid > 0 {
            let client = token::Client::new(&env, &event.token);
            // contract pays itself out — no external signature needed for this leg
            let contract_addr = env.current_contract_address();
            client.transfer(&contract_addr, &attendee, &paid);
        }

        event.registered -= 1;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        env.storage()
            .persistent()
            .remove(&DataKey::Registered(event_id, attendee));
    }

    pub fn payout(env: Env, organizer: Address, event_id: u32) {
        organizer.require_auth();
        let event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .unwrap();
        assert!(caller_is_organizer(&event, &organizer), "not the organizer");

        let client = token::Client::new(&env, &event.token);
        let contract_addr = env.current_contract_address();
        let balance = client.balance(&contract_addr);
        if balance > 0 {
            client.transfer(&contract_addr, &organizer, &balance);
        }
    }
}

fn caller_is_organizer(event: &Event, caller: &Address) -> bool {
    &event.organizer == caller
}

mod test;
