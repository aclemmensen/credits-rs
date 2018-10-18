extern crate serde;
extern crate serde_json;
extern crate uuid;
extern crate stopwatch;

#[macro_use]
extern crate serde_derive;
use std::collections::HashMap;
use stopwatch::Stopwatch;
use uuid::prelude::*;
use CreditEvent::*;

trait Aggregate {
    type Item;
    type Cmd;
    type Error;

    fn version(&self) -> u64;
    fn handle(&self, cmd: &Self::Cmd) -> Result<Vec<Self::Item>, Self::Error>;
    fn apply(self, evt: &Self::Item) -> Self where Self: Sized;
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Contract {
    version: u64,
    amount: i64,
    reservations: HashMap<Uuid, CreditReservation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreditReservation {
    amount: i64,
}

#[derive(Debug)]
enum CreditError {
    NotEnoughMoney {has: i64, needs: i64},
    ReservationAlreadyExists,
    ReservationNotFound
}

type R = Result<Vec<CreditEvent>, CreditError>;

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

        Ok(vec![CreditsReserved { amount, id }])
    }

    fn allocate_credits(&self, id: Uuid) -> R {
        match self.reservations.get(&id) {
            Some(res) => Ok(vec![CreditsAllocated {id, amount: res.amount}]),
            None => Err(CreditError::ReservationNotFound)
        }
    }

    fn cancel_reservation(&self, id: Uuid) -> R {
        match self.reservations.get(&id) {
            Some(res) => Ok(vec![ReservationCancelled(id, res.amount)]),
            None => Err(CreditError::ReservationNotFound)
        }
    }

    fn apply_all(self, evts: &Vec<CreditEvent>) -> Self {
        evts.iter().fold(self, |s, ref e| s.apply(e))
    }
}

impl Aggregate for Contract {
    type Item = CreditEvent;
    type Cmd = CreditCommand;
    type Error = CreditError;

    fn version(&self) -> u64 {
        self.version
    }

    fn handle(&self, cmd: &Self::Cmd) -> Result<Vec<Self::Item>, Self::Error> {
        match cmd {
            &CreditCommand::AddCredits(amt) => self.add_credits(amt),
            &CreditCommand::ReserveCredits(amt, id) => self.reserve_credits(amt, id),
            &CreditCommand::AllocateCredits(id) => self.allocate_credits(id),
            &CreditCommand::CancelReservation(id) => self.cancel_reservation(id)
        }
    }

    fn apply(mut self, evt: &Self::Item) -> Self {
        self.version += 1;

        match evt {
            CreditsAdded(v) => self.amount += v,
            &CreditsReserved { id, amount } => {
                self.reservations.insert(id, CreditReservation { amount });
                self.amount -= amount;
            },
            &CreditsAllocated {id, amount: _} => {
                self.reservations.remove(&id);
            },
            &ReservationCancelled(id, amount) => {
                self.reservations.remove(&id);
                self.amount += amount;
            }
        };

        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum CreditEvent {
    CreditsAdded(i64),
    CreditsReserved { amount: i64, id: Uuid },
    CreditsAllocated {id: Uuid, amount: i64 },
    ReservationCancelled(Uuid, i64)
}

#[derive(Debug, Serialize, Deserialize)]
enum CreditCommand {
    AddCredits(i64),
    ReserveCredits(i64, Uuid),
    AllocateCredits(Uuid),
    CancelReservation(Uuid)
}

fn run_cmd(c: &mut Contract, cmd: CreditCommand) -> Result<Vec<CreditEvent>, CreditError> {
    let x = c.handle(&cmd);
    match x {
        Ok(evts) => {
            evts.iter().fold(c, |mut s, ref e| s.apply(e));
            Ok(evts)
        },
        Err(e) => Err(e)
    }
}

fn main() {
    let seed = CreditsAdded(10000000);
    
    let mut a: Contract = Contract::default();
    a = a.apply(&seed);

    let mut all_events = vec![seed];

    loop {
        let id = Uuid::new_v4();
        if let Ok(evts) = run_cmd(&mut a, CreditCommand::ReserveCredits(10, id)) {
            all_events.extend(evts);
        }
        // if let Ok(evts) = a.handle(&CreditCommand::ReserveCredits(10, id)) {
        //     a = a.apply_all(&evts);
        //     all_events.extend(evts);
        //     if let Ok(evts2) = a.handle(&CreditCommand::AllocateCredits(id)) {
        //         a = a.apply_all(&evts2);
        //         all_events.extend(evts2);
        //     } else {
        //         break;
        //     }
        // } else {
        //     break;
        // }
    }

    let sw = Stopwatch::start_new();
    let b: Contract = Contract::default();
    let b = b.apply_all(&all_events);
    let elapsed = sw.elapsed();

    println!("{:?} {}", elapsed, serde_json::to_string_pretty(&b).unwrap());


    // let cmds = vec![
    //     CreditCommand::AddCredits(100),
    //     CreditCommand::ReserveCredits(36, Uuid::new_v4()),
    //     CreditCommand::ReserveCredits(21, Uuid::new_v4())
    // ];

    // let wat = cmds.iter().fold(a, |c, ref cmd| {
    //     let evts = c.handle(cmd).unwrap();
    //     evts.iter().fold(c, |c, ref e| c.apply(e))
    // });

    // println!("{}", serde_json::to_string(&a).unwrap());
}
