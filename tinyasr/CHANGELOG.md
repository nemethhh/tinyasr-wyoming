# Changelog

## 0.3.0

- Any tinyasr model: new `model` option (`en-ru-uk` built in; `en-es-it-pt`, `en-fa`, `en-km`, `uz` downloaded once
  on first start; `custom` with `custom_model`, a `.tzf` or `.tzf.gz` under `/share` or `/media`). Home Assistant is
  offered the model's own languages.

## 0.2.0

- New `mode` option, `whole` by default: Home Assistant ends the request, then tinyasr transcribes it in one pass.
  Works on low-power CPUs (tested on an Intel Celeron N4100, about 1x the request's length). `live` is the 0.1.0
  behaviour, for CPUs fast enough to keep up.
- `threads: 0` (default): all cores, at most 8. Default `beam` is now 1.
- A beam search that finds no words is retried greedy.

## 0.1.0

- First release: tinyasr `en-ru-uk` as a Wyoming speech-to-text service, end of speech by tinyasr's end-of-turn head.
