# chilpa-pg-radio

Internet radio for Chilpa, as a plugin: browse stations by country, genre and
language, search them by name, and play one.

It reads an open, community-kept station directory that asks for no account.
It hands Chilpa the station's stream address; Chilpa does the playing.

## What it does

- The root offers a search by name and four folders: the most listened to,
  by country, by genre and by language.
- A station that only broadcasts in HLS is listed and says why it does not
  play.
- The directory runs on mirrors. They are tried in order.
- Its own few words come in English and Spanish; Chilpa says which.

## Building

```
rustup target add wasm32-unknown-unknown   # once per machine
tools/build-plugin . <out dir>             # leaves <out>/app.chilpa.radio/
tools/install-dev <out>/app.chilpa.radio <XDG_DATA_HOME>
cargo test                                 # against recorded replies, no network
```

The SDK comes from [chilpa-plugins](https://github.com/vicrodh/chilpa-plugins),
pinned to the `protocol-4` tag in `Cargo.toml`.
