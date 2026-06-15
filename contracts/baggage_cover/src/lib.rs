#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

// ---------------------------------------------------------------------------
// Status codes returned by the contract
// ---------------------------------------------------------------------------
const STATUS_NONE: u32 = 0; // placeholder, not used as a stored state
const STATUS_ACTIVE: u32 = 1; // policy bought, waiting for the flight
const STATUS_LOST: u32 = 2; // airline (oracle) confirmed the bag is lost
const STATUS_CLAIMED: u32 = 3; // passenger has claimed the payout
const STATUS_CANCELLED: u32 = 4; // passenger cancelled the policy

// ---------------------------------------------------------------------------
// Storage keys and stored value shapes
// ---------------------------------------------------------------------------
#[contracttype]
pub enum DataKey {
    /// Admin address (set once in `init`).
    Admin,
    /// Airline address authorized to report losses for a given flight.
    Airline(Symbol),
    /// A single coverage policy, keyed by (flight_id, baggage_tag).
    Policy(Symbol, Symbol),
}

#[contracttype]
pub struct Policy {
    pub passenger: Address,
    pub airline: Address,
    pub flight_id: Symbol,
    pub baggage_tag: Symbol,
    pub coverage_amount: u64,
    pub status: u32,
    pub evidence_hash: Symbol,
    pub purchased_at: u64,
    pub reported_at: u64,
    pub reason: Symbol,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------
#[contract]
pub struct BaggageCover;

#[contractimpl]
impl BaggageCover {
    // -----------------------------------------------------------------------
    /// Initialize the contract. Stores the admin address that will authorize
    /// airlines. Must be called exactly once before any other function.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    // -----------------------------------------------------------------------
    /// Authorize an airline to be the trusted reporter for a specific
    /// flight. Only the admin stored by `init` may call this.
    pub fn authorize_airline(env: Env, flight_id: Symbol, airline: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Airline(flight_id), &airline);
    }

    // -----------------------------------------------------------------------
    /// Buy a baggage coverage policy for a given flight and bag tag.
    ///
    /// The passenger authorizes the call, the flight must have an authorized
    /// airline, the (flight_id, baggage_tag) pair must be unique, and
    /// `coverage_amount` must be greater than zero. The new policy is stored
    /// in `STATUS_ACTIVE` state. No real XLM is moved; the coverage amount
    /// is recorded for later accounting.
    pub fn buy_coverage(
        env: Env,
        passenger: Address,
        flight_id: Symbol,
        baggage_tag: Symbol,
        coverage_amount: u64,
    ) -> u32 {
        passenger.require_auth();

        if coverage_amount == 0 {
            panic!("Coverage amount must be greater than zero");
        }

        let airline: Address = env
            .storage()
            .instance()
            .get(&DataKey::Airline(flight_id.clone()))
            .expect("Flight is not registered with an authorized airline");

        let key = DataKey::Policy(flight_id.clone(), baggage_tag.clone());
        if env.storage().instance().has(&key) {
            panic!("A policy for this flight and baggage tag already exists");
        }

        let policy = Policy {
            passenger: passenger.clone(),
            airline,
            flight_id: flight_id.clone(),
            baggage_tag: baggage_tag.clone(),
            coverage_amount,
            status: STATUS_ACTIVE,
            evidence_hash: Symbol::new(&env, ""),
            purchased_at: env.ledger().timestamp(),
            reported_at: 0,
            reason: Symbol::new(&env, ""),
        };
        env.storage().instance().set(&key, &policy);

        STATUS_ACTIVE
    }

    // -----------------------------------------------------------------------
    /// Airline (or trusted oracle) reports that a specific bag was lost on
    /// a specific flight. The caller must be the airline that was previously
    /// authorized for that flight by the admin. The policy must currently be
    /// in `STATUS_ACTIVE`. On success, the policy transitions to
    /// `STATUS_LOST` and stores the evidence hash and the report time.
    pub fn report_lost(
        env: Env,
        airline: Address,
        flight_id: Symbol,
        baggage_tag: Symbol,
        evidence_hash: Symbol,
    ) -> u32 {
        airline.require_auth();

        let registered: Address = env
            .storage()
            .instance()
            .get(&DataKey::Airline(flight_id.clone()))
            .expect("Flight is not registered with an authorized airline");
        if registered != airline {
            panic!("Caller is not the authorized airline for this flight");
        }

        let key = DataKey::Policy(flight_id.clone(), baggage_tag.clone());
        let mut policy: Policy = env
            .storage()
            .instance()
            .get(&key)
            .expect("No policy found for this flight and baggage tag");

        if policy.status != STATUS_ACTIVE {
            panic!("Policy is not in an active state");
        }

        policy.status = STATUS_LOST;
        policy.evidence_hash = evidence_hash;
        policy.reported_at = env.ledger().timestamp();
        env.storage().instance().set(&key, &policy);

        STATUS_LOST
    }

    // -----------------------------------------------------------------------
    /// Passenger claims the payout for a lost bag. The caller must be the
    /// original policy holder, and the policy must be in `STATUS_LOST`.
    /// On success, the policy transitions to `STATUS_CLAIMED` and the
    /// recorded coverage amount is returned. No real XLM transfer occurs;
    /// the amount is the value the off-chain payout layer should disburse.
    pub fn claim_payout(
        env: Env,
        passenger: Address,
        flight_id: Symbol,
        baggage_tag: Symbol,
    ) -> u64 {
        passenger.require_auth();

        let key = DataKey::Policy(flight_id.clone(), baggage_tag.clone());
        let mut policy: Policy = env
            .storage()
            .instance()
            .get(&key)
            .expect("No policy found for this flight and baggage tag");

        if policy.passenger != passenger {
            panic!("Caller is not the policy holder");
        }
        if policy.status != STATUS_LOST {
            panic!("Baggage loss has not been confirmed for this policy");
        }

        let amount = policy.coverage_amount;
        policy.status = STATUS_CLAIMED;
        env.storage().instance().set(&key, &policy);

        amount
    }

    // -----------------------------------------------------------------------
    /// Passenger cancels a still-active policy before the bag has been
    /// reported lost. The caller must be the original policy holder, the
    /// policy must be in `STATUS_ACTIVE`, and a free-form reason is stored
    /// for audit purposes. On success, the policy transitions to
    /// `STATUS_CANCELLED`.
    pub fn cancel(
        env: Env,
        passenger: Address,
        flight_id: Symbol,
        baggage_tag: Symbol,
        reason: Symbol,
    ) -> u32 {
        passenger.require_auth();

        let key = DataKey::Policy(flight_id.clone(), baggage_tag.clone());
        let mut policy: Policy = env
            .storage()
            .instance()
            .get(&key)
            .expect("No policy found for this flight and baggage tag");

        if policy.passenger != passenger {
            panic!("Caller is not the policy holder");
        }
        if policy.status != STATUS_ACTIVE {
            panic!("Only active policies can be cancelled");
        }

        policy.status = STATUS_CANCELLED;
        policy.reason = reason;
        env.storage().instance().set(&key, &policy);

        STATUS_CANCELLED
    }

    // -----------------------------------------------------------------------
    /// Read-only view: returns the current status code of a policy
    /// (1 = Active, 2 = Lost, 3 = Claimed, 4 = Cancelled).
    pub fn get_status(env: Env, flight_id: Symbol, baggage_tag: Symbol) -> u32 {
        let key = DataKey::Policy(flight_id, baggage_tag);
        let policy: Policy = env
            .storage()
            .instance()
            .get(&key)
            .expect("No policy found for this flight and baggage tag");
        policy.status
    }

    // -----------------------------------------------------------------------
    /// Read-only view: returns `true` if a policy exists for the given
    /// flight and baggage tag, the loss has been confirmed, and the
    /// passenger has not yet claimed the payout (i.e. the policy is in
    /// `STATUS_LOST` and therefore payable).
    pub fn is_payable(env: Env, flight_id: Symbol, baggage_tag: Symbol) -> bool {
        let key = DataKey::Policy(flight_id, baggage_tag);
        match env.storage().instance().get::<DataKey, Policy>(&key) {
            Some(p) => p.status == STATUS_LOST,
            None => false,
        }
    }
}
