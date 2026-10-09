<p align="center">
  <img src="docs/assets/rerac-logo.png" alt="ReRAC" width="480">
</p>

<h1 align="center">ReRAC</h1>

<p align="center">An unofficial native PC port of Ratchet & Clank (2002).</p>

<p align="center">
  <a href="https://discord.gg/v2Ek44kdyu">Discord</a> ·
  <a href="https://re-rac.github.io">Website</a> ·
  <a href="https://github.com/re-rac/rerac-launcher">Launcher</a>
</p>

## Please read first

> [!IMPORTANT]
> This repository is for developing ReRAC, not for playing it. Players will install and run the game through the
> ReRAC Launcher, which has not been released yet. Follow the project at [re-rac.github.io](https://re-rac.github.io).

> [!WARNING]
> You need your own copy of the game: no game assets are included here. ReRAC currently supports one disc, listed on
> the [supported versions](https://re-rac.github.io/docs/supported-versions/) page. ReRAC is an unofficial fan project
> and is not affiliated with or endorsed by Sony Interactive Entertainment or Insomniac Games.

## About

ReRAC rebuilds Ratchet & Clank (2002, PlayStation 2) as a native PC game, written in Rust on the Bevy engine. The
game's systems are ported and run natively, with no emulator in between. The goal is a port
that looks, sounds and plays like the original.

## Status

ReRAC is in early, active development and is not yet playable from start to finish. All levels load and render, and
more of the game comes to life with each update. See the [status page](https://re-rac.github.io/status/) for what
works today.

## Building from source

You need Rust, installed through [rustup](https://rustup.rs), and an image (`.iso`) of your own disc. macOS on Apple
Silicon is the only platform tested so far. Run every command from the repository root.

First, extract the game data from your disc into `extracted/`. This builds `rerac-extract`, checks that your disc is
supported, and verifies every extracted file:

```sh
cargo xtask regen-data --iso "<path to your disc image>.iso"
```

Then build and run the game:

```sh
cargo dev
```

The first build takes a few minutes; later rebuilds take seconds. Tests run through `cargo xtask`:

```sh
cargo xtask test-quick      # fast unit tests
cargo xtask test-job hero   # unit tests plus one area's integration tests
cargo xtask test-full       # the full suite
```

More detail is in [`docs/workflows/`](docs/workflows/), for example
[`game-data.md`](docs/workflows/game-data.md) for the extractor and
[`dev-switches.md`](docs/workflows/dev-switches.md) for debugging options.

## Repository layout

| Folder | Contents |
|---|---|
| `crates/` | The game itself: file formats, game logic, the engine (`rerac`) and the extractor (`rerac-extract`) |
| `tools/` | Development tools that never ship, including `cargo xtask` |
| `docs/` | [Developer documentation](docs/): format specs, plans and workflows |

## Related

- [ReRAC Launcher](https://github.com/re-rac/rerac-launcher), which installs and runs the game.
- [re-rac.github.io](https://re-rac.github.io), the project website.

## License

ReRAC is released under the ISC license. See [LICENSE](LICENSE).
