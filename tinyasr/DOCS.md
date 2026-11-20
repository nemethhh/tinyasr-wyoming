# tinyasr

Speech to text for Home Assistant's Assist with [tinyasr](https://github.com/dkaraush/tinyasr): a small,
dependency-free recognizer (about 65 MB of RAM in use) for **English, Russian and Ukrainian**.

## Setup

1. Install and start the add-on. Home Assistant finds it: accept the **Wyoming Protocol** integration it offers
   (Settings → Devices & services).
2. Settings → Voice assistants → your assistant → **Speech-to-text**: pick **tinyasr**, and the assistant's language.

Each assistant recognizes in its own language (no language detection).

## When a request ends

By default (`end_of_speech: tinyasr`) the add-on, not Home Assistant, decides when you have finished speaking, and
Home Assistant's own "finished speaking" detection is not used for assistants with tinyasr:

- tinyasr's end-of-turn model judges at each pause whether the sentence is complete. A request ends once it says so
  **and** the pause has lasted `min_pause_seconds`.
- After speech, a pause of `max_pause_seconds` ends the request whatever the model says.
- Nothing said for `no_speech_seconds`: the request ends with no text. No request runs longer than `max_seconds`.
- A pause after a wake word alone ("Alexa." …) does not end the request, for satellites that send the wake word
  along (`wake_words`).

With `end_of_speech: home_assistant`, Home Assistant ends requests as with any other speech to text, and the add-on
only transcribes.

**After changing `end_of_speech`, reload the integration**: Settings → Devices & services → Wyoming Protocol →
tinyasr → ⋮ → Reload (or restart Home Assistant). Home Assistant reads this setting only when it sets up the
service; until the reload it keeps the old one, and with `home_assistant` nobody would end requests early.

## Options

| option | default | |
|---|---|---|
| `beam` | 4 | Beam width of the final pass: 1 is fastest, 4 most accurate. |
| `threads` | 2 | CPU threads per request. |
| `end_of_speech` | `tinyasr` | `tinyasr` or `home_assistant` (see above). |
| `min_pause_seconds` | 0.7 | Shortest pause that can end a request. |
| `max_pause_seconds` | 1.25 | A pause this long always ends it. |
| `no_speech_seconds` | 8 | Give up when nothing is said. |
| `max_seconds` | 30 | Longest request. |
| `wake_words` | Alexa, Echo, Computer, Amazon, Ziggy (en/ru) | A sentence that is only one of these never ends a request. |
| `debug_logging` | false | Log each request's text and the end-of-turn model's scores. |

## Credits

tinyasr and its models: GPL-3.0-or-later, by Dmitrii Karaush. The models were trained mainly on
[YODAS](https://huggingface.co/datasets/espnet/yodas3) (CC BY 3.0). Voice activity detection:
[Silero VAD](https://github.com/snakers4/silero-vad) (MIT).
