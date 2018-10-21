extern crate serde;
extern crate serde_json;
extern crate uuid;
extern crate stopwatch;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate chrono;
extern crate dotenv;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use stopwatch::Stopwatch;
use uuid::prelude::*;
use chrono::prelude::*;
use chrono::Duration;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use CreditEvent::*;

mod eventstore;
use eventstore::{run_and_store, run_and_store_batch, run_cmd};

type R = Result<Vec<CreditEvent>, CreditError>;
type MyPool = Pool<PostgresConnectionManager>;
type Ts = DateTime<Utc>;

trait Aggregate {
    type Item;
    type Cmd;
    type Error;

    fn id(&self) -> i64;
    fn version(&self) -> i64;
    fn handle(&self, cmd: &Self::Cmd) -> Result<Vec<Self::Item>, Self::Error>;
    fn apply(&mut self, evt: &Self::Item) -> ();
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Contract {
    id: i64,
    version: i64,
    amount: Amount,
    spent: Amount,
    reservations: HashMap<Uuid, CreditReservation>,
    allocations: HashMap<Uuid, CreditReservation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreditReservation {
    amount: Amount,
    created_time: Ts,
    allocated_time: Option<Ts>
}

#[derive(Debug)]
pub enum CreditError {
    NotEnoughMoney {has: Amount, needs: Amount},
    ReservationAlreadyExists,
    ReservationNotFound,
    AllocationNotFound,

    ConcurrencyError,
    StorageError(postgres::error::Error),
    DataError(serde_json::Error)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreditEvent {
    CreditsAdded(Amount),
    CreditsReserved { 
        amount: Amount,
        id: Uuid,
        timestamp: Ts
    },
    CreditsAllocated {
        id: Uuid,
        amount: Amount,
        timestamp: Ts
    },
    ReservationCancelled(Uuid, Amount),
    ReservationExpired {
        id: Uuid,
        amount_freed: Amount,
        available: Amount
    },
    AllocationFreed {
        id: Uuid,
        amount: Amount,
        available: Amount
    },
    ReservationSpent {
        id: Uuid,
        amount: Amount
    }
}

type PersonId = i64;
type Amount = i64;

#[derive(Debug, Serialize, Deserialize)]
pub enum CreditCommand {
    // Add credits to account
    AddCredits(Amount),
    // Reserve an amount of credits
    ReserveCredits(Amount, Uuid),
    // Allocate credits reserved with a reservation
    AllocateCredits(Uuid),
    // Cancel a reservation made previously
    CancelReservation(Uuid),
    // Clean up expired reservations older than the provided number of seconds
    EvictExpiredReservations(i64),
    // Free an allocation made previously
    FreeAllocation(Uuid),
    // Permanently spend credits
    SpendReservation(Uuid)
}

impl Contract {
    
    fn add_credits(&self, amount: i64) -> R {
        Ok(vec![CreditsAdded(amount)])
    }

    fn reserve_credits(&self, amount: i64, id: Uuid) -> R {
        if self.amount-amount < 0 {
            return Err(CreditError::NotEnoughMoney {has: self.amount, needs: amount-self.amount })
        }

        if self.reservations.contains_key(&id) {
            return Err(CreditError::ReservationAlreadyExists)
        }

        Ok(vec![CreditsReserved { amount, id, timestamp: Utc::now() }])
    }

    fn allocate_credits(&self, id: Uuid) -> R {
        match self.reservations.get(&id) {
            Some(res) => Ok(vec![CreditsAllocated {
                id,
                amount: res.amount,
                timestamp: Utc::now()
            }]),
            None => Err(CreditError::ReservationNotFound)
        }
    }

    fn cancel_reservation(&self, id: Uuid) -> R {
        match self.reservations.get(&id) {
            Some(res) => Ok(vec![ReservationCancelled(id, res.amount)]),
            None => Err(CreditError::ReservationNotFound)
        }
    }

    fn evict_expired_resevations(&self, age: i64) -> R {
        let now = Utc::now();
        let mut total_freed = 0;
        let dur = Duration::seconds(age);
        let events: Vec<CreditEvent> = self.reservations.iter().filter_map(|(id, r)| {
            if (r.created_time+dur) < now {
                total_freed += r.amount;
                Some(ReservationExpired {
                    id: *id,
                    amount_freed: r.amount,
                    available: self.amount + total_freed
                })
            } else {
                None
            }
        }).collect();
        
        Ok(events)
    }

    fn free_allocation(&self, id: Uuid) -> R {
        if let Some(res) = self.allocations.get(&id) {
            return Ok(vec![AllocationFreed {
                id,
                amount: res.amount,
                available: self.amount + res.amount
            }]);
        }

        Err(CreditError::AllocationNotFound)
    }

    fn spend_reservation(&self, id: Uuid) -> R {
        match self.reservations.get(&id) {
            Some(res) => Ok(vec![ReservationSpent {
                id,
                amount: res.amount
            }]),
            None => Err(CreditError::ReservationNotFound)
        }
    }
}

impl Aggregate for Contract {
    type Item = CreditEvent;
    type Cmd = CreditCommand;
    type Error = CreditError;

    fn version(&self) -> i64 {
        self.version
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn handle(&self, cmd: &Self::Cmd) -> Result<Vec<Self::Item>, Self::Error> {
        match cmd {
            &CreditCommand::AddCredits(amt) => self.add_credits(amt),
            &CreditCommand::ReserveCredits(amt, id) => self.reserve_credits(amt, id),
            &CreditCommand::AllocateCredits(id) => self.allocate_credits(id),
            &CreditCommand::CancelReservation(id) => self.cancel_reservation(id),
            &CreditCommand::EvictExpiredReservations(age) => self.evict_expired_resevations(age),
            &CreditCommand::FreeAllocation(id) => self.free_allocation(id),
            &CreditCommand::SpendReservation(id) => self.spend_reservation(id)
        }
    }

    fn apply(&mut self, evt: &Self::Item) -> () {
        self.version += 1;

        match evt {
            CreditsAdded(v) => self.amount += v,
            &CreditsReserved { id, amount, timestamp } => {
                self.reservations.insert(id, CreditReservation { amount, created_time: timestamp, allocated_time: None });
                self.amount -= amount;
            },
            &CreditsAllocated {id, amount: _, timestamp} => {
                let mut res = self.reservations.remove(&id).unwrap();
                res.allocated_time = Some(timestamp);
                self.allocations.insert(id, res);
            },
            &ReservationCancelled(id, amount) => {
                self.reservations.remove(&id).unwrap();
                self.amount += amount;
            },
            &ReservationExpired {id, amount_freed, available: _} => {
                self.reservations.remove(&id).unwrap();
                self.amount += amount_freed;
            },
            &AllocationFreed {id, amount, available: _} => {
                self.amount += amount;
                self.allocations.remove(&id).unwrap();
            },
            &ReservationSpent {id, amount: amt } => {
                self.reservations.remove(&id).unwrap();
                self.spent += amt;
            }
        };
    }
}

fn main2() -> Result<(), CreditError> {
    let pool = eventstore::pool();
    eventstore::init(pool.clone());
    let sw = Stopwatch::start_new();
    let mut c = eventstore::load(5, &pool)?;
    info!("Loaded agg in {} ms", sw.elapsed_ms());

    run_and_store(&mut c, CreditCommand::AddCredits(10000), &pool)?;
    loop {
        let uuid = Uuid::new_v4();
        let r = run_and_store_batch(&mut c, vec![
            CreditCommand::ReserveCredits(10, uuid),
            // CreditCommand::AllocateCredits(uuid),
            CreditCommand::SpendReservation(uuid),
            CreditCommand::EvictExpiredReservations(60)
        ], &pool);
        match r {
            Ok(_) => {},
            Err(e) => {
                error!("Error: {:?}", e);
                break;
            }
        }
        
    }

    // println!("{:?}", c);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_empty_by_default() {
        let c = Contract::default();
        assert_eq!(c.amount, 0);
        assert_eq!(c.id, 0);
    }

    #[test]
    fn it_updates_amount() {
        let c = with_amount(10);
        assert_eq!(c.amount, 10);
    }

    #[test]
    fn it_reserves_amount() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(5, id)).unwrap();
        assert_eq!(c.amount, 5);
    }

    #[test]
    fn it_cannot_reserve_too_much() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        let r = run_cmd(&mut c, CreditCommand::ReserveCredits(20, id));
        r.expect_err("should error out");
    }

    #[test]
    fn it_spends_what_is_available() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(10, id)).unwrap();
        run_cmd(&mut c, CreditCommand::SpendReservation(id)).unwrap();
        assert_eq!(c.amount, 0);
        assert_eq!(c.spent, 10);
    }

    #[test]
    fn it_cannot_respend() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(5, id)).unwrap();
        run_cmd(&mut c, CreditCommand::SpendReservation(id)).unwrap();
        run_cmd(&mut c, CreditCommand::SpendReservation(id))
            .expect_err("should not allow respend");
    }

    #[test]
    fn it_frees_reservation() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(5, id)).unwrap();
        run_cmd(&mut c, CreditCommand::CancelReservation(id)).unwrap();
        assert_eq!(c.amount, 10);
    }

    #[test]
    fn cancelled_reservation_cannot_be_allocated() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(5, id)).unwrap();
        run_cmd(&mut c, CreditCommand::CancelReservation(id)).unwrap();
        run_cmd(&mut c, CreditCommand::AllocateCredits(id))
            .expect_err("should not allow allocation");        
    }

    #[test]
    fn spent_reservation_cannot_be_allocated() {
        let mut c = with_amount(10);
        let id = Uuid::new_v4();
        run_cmd(&mut c, CreditCommand::ReserveCredits(5, id)).unwrap();
        run_cmd(&mut c, CreditCommand::SpendReservation(id)).unwrap();
        run_cmd(&mut c, CreditCommand::AllocateCredits(id))
            .expect_err("should not allow allocation");        
    }

    fn with_amount(amount: Amount) -> Contract {
        let mut c = Contract::default();
        run_cmd(&mut c, CreditCommand::AddCredits(amount)).unwrap();
        c
    }
}

#[allow(unused)]
fn benchmark() {
    let seed = CreditsAdded(10000000);
    
    let mut a: Contract = Contract::default();
    a.apply(&seed);

    let mut all_events = vec![seed];

    loop {
        let id = Uuid::new_v4();

        if let Ok(evts) = run_cmd(&mut a, CreditCommand::ReserveCredits(10, id)) {
            all_events.extend(evts);
            if let Ok(evts2) = run_cmd(&mut a, CreditCommand::AllocateCredits(id)) {
                all_events.extend(evts2);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let sw = Stopwatch::start_new();
    let mut b: Contract = Contract::default();
    for e in all_events {
        b.apply(&e);
    }
    let elapsed = sw.elapsed();

    println!("{:?} {}", elapsed, serde_json::to_string_pretty(&b).unwrap());
}

fn main() {
    env_logger::init();
    main2().unwrap();
    // benchmark();
}
