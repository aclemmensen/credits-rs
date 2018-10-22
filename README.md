# Credits

A small event-sourced system for managing fake credits. Uses Postgres for event storage and snapshots.

## How to run

Start a Postgres DB if you don't have one:

```sh
mkdir data
docker run -d \
  --name postgres \
  --net=host \
  -e POSTGRES_PASSWORD=credits \
  -e POSTGRES_USER=credits \
  -e PGDATA=/data \
  -v $(pwd):/data \
  -e POSTGRES_DB=credits\
   postgres:11
```

Run the program:

```sh
export RUST_LOG=credits=info
cargo run --release
```

This will start the gRPC server running on port 5951. You can access it with grpcc like so:

```sh
make grpcc
```

## Concepts

Aggregates are containers for state and embed business logic. They are responsible for ensuring
only valid state changes happen. They fully contain their state but emit events describing state
changes that others may depend upon.

Any state change is carried out through four stages:

- Command describing your intent (imperative)
- Event describing the effect of said change (past participle)
- Handler function carries out validation and creates one or more events
- Apply function that carries out the local state change in response to events 
