#![no_std]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_borrows_for_generic_args)]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env};

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

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    EventNotFound = 1,
    EventFull = 2,
    NotRegistered = 3,
    RefundNotAllowed = 4,
    DeadlinePassed = 5,
    NotOrganizer = 6,
    OnlyAttendeeOrOrganizer = 7,
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

    pub fn check_in(env: Env, organizer: Address, event_id: u32, attendee: Address) {
        organizer.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::CheckedIn(event_id, attendee), &true);
    }

    pub fn register(env: Env, attendee: Address, event_id: u32) -> Result<(), ContractError> {
        attendee.require_auth();
        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(ContractError::EventNotFound)?;
        if event.registered >= event.capacity {
            return Err(ContractError::EventFull);
        }

        if event.price > 0 {
            let client = token::Client::new(&env, &event.token);
            // pay INTO the contract's own balance, not the organizer, so refunds
            // don't require the organizer's live signature later.
            client.transfer(&attendee, &env.current_contract_address(), &event.price);
        }

        event.registered += 1;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        env.storage()
            .persistent()
            .set(&DataKey::Registered(event_id, attendee), &event.price);
        Ok(())
    }

    pub fn refund(
        env: Env,
        caller: Address,
        event_id: u32,
        attendee: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(ContractError::EventNotFound)?;
        let paid: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Registered(event_id, attendee.clone()))
            .ok_or(ContractError::NotRegistered)?;

        let is_organizer = caller == event.organizer;
        let is_self = caller == attendee;

        if is_self {
            if !event.self_refund_allowed {
                return Err(ContractError::RefundNotAllowed);
            }
            if event.refund_deadline > 0 && env.ledger().timestamp() >= event.refund_deadline {
                return Err(ContractError::DeadlinePassed);
            }
        } else {
            if !is_organizer {
                return Err(ContractError::OnlyAttendeeOrOrganizer);
            }
        }

        if paid > 0 {
            let client = token::Client::new(&env, &event.token);
            // contract pays itself out — no external signature needed for this leg
            client.transfer(&env.current_contract_address(), &attendee, &paid);
        }

        event.registered -= 1;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
        env.storage()
            .persistent()
            .remove(&DataKey::Registered(event_id, attendee));
        Ok(())
    }

    pub fn payout(env: Env, organizer: Address, event_id: u32) -> Result<(), ContractError> {
        organizer.require_auth();
        let event: Event = env
            .storage()
            .persistent()
            .get(&DataKey::Event(event_id))
            .ok_or(ContractError::EventNotFound)?;
        if !caller_is_organizer(&event, &organizer) {
            return Err(ContractError::NotOrganizer);
        }

        let client = token::Client::new(&env, &event.token);
        let balance = client.balance(&env.current_contract_address());
        if balance > 0 {
            client.transfer(&env.current_contract_address(), &organizer, &balance);
        }
        Ok(())
    }
}

fn caller_is_organizer(event: &Event, caller: &Address) -> bool {
    &event.organizer == caller
}

mod test;
