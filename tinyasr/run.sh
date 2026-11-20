#!/usr/bin/with-contenv bashio
# shellcheck shell=bash
set -e

args=(
  --listen 0.0.0.0:10300
  --models /models
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
