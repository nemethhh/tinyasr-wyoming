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
