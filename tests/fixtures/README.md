# Test audio

Synthetic speech made with [Piper](https://github.com/OHF-Voice/piper1-gpl), 16 kHz mono 16-bit PCM. Each file starts
with 0.8 s of silence, as an Echo running echo-dot-assist sends it.

| file | voice | text |
|---|---|---|
| `en_command.wav` | `en_US-ljspeech-medium` | "Turn on the kitchen light." + 4 s silence |
| `ru_command.wav` | `ru_RU-dmitri-medium` | "Включи свет на кухне." + 4 s silence |
| `uk_command.wav` | `uk_UA-ukrainian_tts-medium` | "Увімкни світло на кухні." + 4 s silence |
| `en_midpause.wav` | `en_US-ljspeech-medium` | "Set a timer for" + 0.6 s + "ten minutes." + 4 s silence |
| `en_wake_pause.wav` | `en_US-ljspeech-medium` | "Alexa." + 1.0 s + "What time is it?" + 4 s silence |
| `silence.wav` | — | 12 s of digital silence |

Voice training data: LJ Speech (public domain), OHF-Voice voice-datasets `dmitri` and `ukrainian_tts` (CC0).
