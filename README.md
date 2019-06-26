# LifeShot IO

*This is a work in progress*

## Play

You can play the game on [lifeshot.io](https://lifeshot.io)

## Build

To build from source and run, install [Rust](https://rust-lang.org), then:

```shell
$ cargo run --release -- --addr ws://server.lifeshot.io:1154
```

To run with local server:

```shell
$ cargo run --release -- --host 127.0.0.1 --port 1154 --addr ws://127.0.0.1:1154 with-server
```

To build web version, `cargo-web` is needed:

```shell
$ cargo install cargo-web
$ cargo web start --release --open
```