CARGO ?= cargo
export TINYASR_MODELS ?= $(CURDIR)/models
# the tinyasr commit Cargo.lock pins; the model files come from the same commit
TINYASR_REV := $(shell sed -n 's|.*nemethhh/tinyasr?branch=wyoming-pause-end\#\([0-9a-f]*\)".*|\1|p' Cargo.lock)
MODEL_URL := https://raw.githubusercontent.com/nemethhh/tinyasr/$(TINYASR_REV)/models
MODELS := models/en-ru-uk.tzf models/silero_vad.onnx

.PHONY: test models release image smoke

models: $(MODELS)

models/%: Cargo.lock
	@test -n "$(TINYASR_REV)" || { echo "no tinyasr commit in Cargo.lock"; exit 1; }
	mkdir -p models
	curl -fsSL $(MODEL_URL)/$*.gz | gunzip > $@.tmp && mv $@.tmp $@

test: models
	$(CARGO) test --release

release:
	$(CARGO) build --release

image:
	docker build -f tinyasr/Dockerfile --build-arg BUILD_FROM=ghcr.io/home-assistant/amd64-base:3.21 -t tinyasr-addon .

smoke:
	docker rm -f tinyasr-smoke >/dev/null 2>&1 || true
	docker run -d --name tinyasr-smoke -p 127.0.0.1:10399:10300 --entrypoint /usr/bin/tinyasr-wyoming tinyasr-addon --models /models
	sleep 2
	uv run -q --no-project --with wyoming python tools/wyoming_client.py 127.0.0.1 10399 tests/fixtures/ru_command.wav ru
	docker logs tinyasr-smoke
	docker rm -f tinyasr-smoke >/dev/null
