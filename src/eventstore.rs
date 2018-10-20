use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use r2d2::Pool;
use {MyPool, CreditEvent, CreditError, Contract, Aggregate};
use serde_json;
use postgres::error;
use std::env;
use dotenv;

pub fn init(pool: MyPool) -> () {
    let conn = pool.get().unwrap();
    conn.batch_execute("
        create table if not exists events (
            id bigserial primary key,
            aggregate_id bigint,
            ts timestamp default current_timestamp,
            payload text,
            version bigint
        );
        
        create index if not exists events_agg on events (aggregate_id);
        
        create table if not exists aggregates (
            id bigint primary key,
            v bigint
        );").unwrap();
}

impl From<error::Error> for CreditError {
    fn from(err: error::Error) -> Self {
        CreditError::StorageError(err)
    }
}

impl From<serde_json::Error> for CreditError {
    fn from(err: serde_json::Error) -> Self {
        CreditError::DataError(err)
    }
}

pub fn save_events(pool: &MyPool, aggregate: i64, expected_version: i64, version: i64, events: Vec<CreditEvent>) -> Result<(), CreditError> {
    if events.len() == 0 {
        return Ok(())
    }

    let conn = pool.get().unwrap();
    let trx = conn.transaction()?;

    // conditionally update version from expected value to its current value.
    // if it fails it means somebody else managed to successfully complete an
    // operation on this aggregate before we could finish ourselves
    let affected = trx.execute(
        "insert into aggregates (id, v) values ($2, $1)
        on conflict(id) do update set v = $1 where aggregates.v = $3",
        &[&version, &aggregate, &expected_version])?;

    if affected == 1 {
        let stmt = trx.prepare("insert into events (aggregate_id, payload, version) values($1, $2, $3)")?;
        
        for evt in events.iter() {
            let ser = serde_json::to_string(evt).unwrap();
            stmt.execute(&[&aggregate, &ser, &version])?;
        }

        trx.commit()?;

        return Ok(())
    } else {
        return Err(CreditError::ConcurrencyError)
    }
}

pub fn load_into(c: &mut Contract, pool: &MyPool) -> Result<(), CreditError> {
    let evts = get_events(c.id(), c.version(), &pool)?;
    for evt in evts {
        c.apply(&evt);
    }
    Ok(())
}

pub fn get_events(id: i64, version: i64, pool: &MyPool) -> Result<Vec<CreditEvent>, CreditError> {
    let conn = pool.get().unwrap();
    let rows = &conn.query(
        "select payload from events where aggregate_id = $1 and version > $2 order by id asc",
        &[&id, &version])?;
    let events = rows.iter().map(|row| {
        let evt: String = row.get("payload");
        let credit_evt: CreditEvent = serde_json::from_str(&evt).unwrap();
        credit_evt
    }).collect();

    Ok(events)
}

pub fn pool() -> MyPool {
    dotenv::dotenv().ok();
    let url = env::var("DATABASE_URL").expect("Must have DATABASE_URL env var or in .env file");
    let manager = PostgresConnectionManager::new(url, TlsMode::None).unwrap();
    let pool = Pool::new(manager).unwrap();
    pool
}