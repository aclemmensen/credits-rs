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
    amount: i64,
    reservations: HashMap<Uuid, CreditReservation>,
    allocations: HashMap<Uuid, CreditReservation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreditReservation {
    amount: i64,
    created_time: Ts,
    allocated_time: Option<Ts>
}

#[derive(Debug)]
pub enum CreditError {
    NotEnoughMoney {has: i64, needs: i64},
    ReservationAlreadyExists,
    ReservationNotFound,
    ConcurrencyError,
    StorageError(postgres::error::Error),
    DataError(serde_json::Error)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreditEvent {
    CreditsAdded(i64),
    CreditsReserved { 
        amount: i64,
        id: Uuid,
        timestamp: Ts
    },
    CreditsAllocated {
        id: Uuid,
        amount: i64,
        timestamp: Ts
    },
    ReservationCancelled(Uuid, i64),
    ReservationExpired {id: Uuid, amount_freed: i64, new_total: i64}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreditCommand {
    AddCredits(i64),
    ReserveCredits(i64, Uuid),
    AllocateCredits(Uuid),
    CancelReservation(Uuid),
    EvictExpiredReservations
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

    fn evict_expired_resevations(&self) -> R {
        let now = Utc::now();
        let dur = Duration::minutes(5);
        let mut total_freed = 0;
        let events: Vec<CreditEvent> = self.reservations.iter().filter_map(|(id, r)| {
            if (r.created_time+dur) < now {
                total_freed += r.amount;
                Some(ReservationExpired {
                    id: *id,
                    amount_freed: r.amount,
                    new_total: self.amount + total_freed
                })
            } else {
                None
            }
        }).collect();
        
        Ok(events)
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
            &CreditCommand::EvictExpiredReservations => self.evict_expired_resevations()
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
                self.reservations.remove(&id);
                self.amount += amount;
            },
            &ReservationExpired {id, amount_freed, new_total: _} => {
                self.reservations.remove(&id);
                self.amount += amount_freed;
            }
        };
    }
}

fn main2() -> Result<(), CreditError> {
    let pool = eventstore::pool();
    eventstore::init(pool.clone());
    let sw = Stopwatch::start_new();
    let mut c = eventstore::load(3, &pool)?;
    info!("Loaded agg in {} ms", sw.elapsed_ms());

    run_and_store(&mut c, CreditCommand::AddCredits(10000), &pool)?;
    loop {
        let uuid = Uuid::new_v4();
        let r = run_and_store_batch(&mut c, vec![
            CreditCommand::ReserveCredits(10, uuid),
            CreditCommand::AllocateCredits(uuid),
            CreditCommand::EvictExpiredReservations
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
