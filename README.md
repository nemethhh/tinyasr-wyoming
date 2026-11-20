# tinyasr for Home Assistant

A [Home Assistant](https://www.home-assistant.io/) add-on that runs [tinyasr](https://github.com/dkaraush/tinyasr)
as a [Wyoming](https://github.com/OHF-Voice/wyoming) speech-to-text service for Assist: **English, Russian and
Ukrainian** in a small, fast, CPU-only recognizer.

[![Add repository to Home Assistant](https://my.home-assistant.io/badges/supervisor_add_addon_repository.svg)](https://my.home-assistant.io/redirect/supervisor_add_addon_repository/?repository_url=https%3A%2F%2Fgithub.com%2Fnemethhh%2Ftinyasr-wyoming)

Or: Settings → Add-ons → Add-on Store → ⋮ → Repositories → `https://github.com/nemethhh/tinyasr-wyoming`.
Then install **tinyasr**; see its [documentation](tinyasr/DOCS.md).

- amd64 and aarch64 (prebuilt images).
- About 65 MB of RAM in use.
- Decides itself when you have finished speaking (tinyasr's end-of-turn model), or leaves that to Home Assistant.

## Development

The Rust server is in `src/`; tests (`cargo test`) need the model files: `make test` fetches them from the pinned
tinyasr commit. `make image` builds the add-on image locally.

## Licence

GPL-3.0-or-later, as tinyasr. Credits: tinyasr and its models (GPL-3.0-or-later; trained mainly on YODAS, CC BY 3.0),
Silero VAD (MIT). Test audio: see [tests/fixtures/README.md](tests/fixtures/README.md).
