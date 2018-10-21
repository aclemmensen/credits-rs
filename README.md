# Credits

A small event-sourced system for managing fake credits. Uses Postgres for event storage and snapshots.

## How to run

Start a Postgres DB if you don't have one:

```sh
mkdir data
docker run --name postgres --net=host -d -e POSTGRES_PASSWORD=credits -e POSTGRES_USER=credits -e PGDATA=/data -v $(pwd):/data -e POSTGRES_DB=credits postgres:11
```

Enable logging:

```sh
export RUST_LOG=credits=info
```

Run the program:

```sh
cargo run --release
```
