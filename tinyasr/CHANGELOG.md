# Changelog

## 0.2.0

- New `mode` option, `whole` by default: Home Assistant ends the request, then tinyasr transcribes it in one pass.
  Works on low-power CPUs (tested on an Intel Celeron N4100, about 1x the request's length). `live` is the 0.1.0
  behaviour, for CPUs fast enough to keep up.
- `threads: 0` (default): all cores, at most 8. Default `beam` is now 1.
- A beam search that finds no words is retried greedy.

## 0.1.0

- First release: tinyasr `en-ru-uk` as a Wyoming speech-to-text service, end of speech by tinyasr's end-of-turn head.
