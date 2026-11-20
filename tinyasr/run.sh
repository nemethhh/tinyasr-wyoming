#!/usr/bin/with-contenv bashio
# shellcheck shell=bash
set -e -o pipefail

# the recognizer: built in, downloaded once from the commit the image was built with, or the user's own file
model=$(bashio::config 'model')
rev=$(cat /models/REV)
case "$model" in
  en-ru-uk)
    asr=/models/en-ru-uk.tzf
    ;;
  custom)
    src=$(bashio::config 'custom_model')
    [ -f "$src" ] || bashio::exit.nok "custom_model: no file at '${src}' (put it under /share or /media)"
    case "$src" in
      *.gz) asr=/data/custom.tzf; gunzip -c "$src" > "$asr.tmp" && mv "$asr.tmp" "$asr" ;;
      *) asr=$src ;;
    esac
    ;;
  *)
    mkdir -p "/data/models/$rev"
    find /data/models -mindepth 1 -maxdepth 1 ! -name "$rev" -exec rm -rf {} +
    asr=/data/models/$rev/$model.tzf
    if [ ! -f "$asr" ]; then
      bashio::log.info "Downloading the ${model} model (once)"
      curl -fsSL "https://raw.githubusercontent.com/nemethhh/tinyasr/${rev}/models/${model}.tzf.gz" | gunzip > "$asr.tmp" \
        || bashio::exit.nok "Could not download the ${model} model"
      mv "$asr.tmp" "$asr"
    fi
    ;;
esac

args=(
  --listen 0.0.0.0:10300
  --asr "$asr"
  --vad /models/silero_vad.onnx
  --mode "$(bashio::config 'mode')"
  --beam "$(bashio::config 'beam')"
  --threads "$(bashio::config 'threads')"
  --end-of-speech "$(bashio::config 'end_of_speech')"
  --min-pause-seconds "$(bashio::config 'min_pause_seconds')"
  --max-pause-seconds "$(bashio::config 'max_pause_seconds')"
  --no-speech-seconds "$(bashio::config 'no_speech_seconds')"
  --max-seconds "$(bashio::config 'max_seconds')"
  --wake-words "$(bashio::config 'wake_words' | paste -sd, -)"
)
if bashio::config.true 'debug_logging'; then
  args+=(--debug)
fi

/usr/bin/tinyasr-wyoming "${args[@]}" &
pid=$!

# announce to Home Assistant once it accepts connections
until (exec 3<>/dev/tcp/127.0.0.1/10300) 2>/dev/null; do
  kill -0 "$pid" 2>/dev/null || wait "$pid"
  sleep 0.2
done
bashio::discovery "wyoming" "{\"uri\": \"tcp://$(hostname):10300\"}" > /dev/null || bashio::log.warning "discovery failed"

wait "$pid"
