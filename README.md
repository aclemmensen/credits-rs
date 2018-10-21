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
