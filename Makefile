.PHONY: gallery gallery-clean gallery-prep tap-formula tap-formula-to-tap

# Render all VHS tapes under docs/vhs into GIFs under docs/gallery
gallery: gallery-prep
	@set -e; \
	for f in docs/vhs/dt-run.tape docs/vhs/dt-diff.tape; do \
	  echo "[VHS] Rendering $$f"; \
	  vhs "$$f"; \
	done

# Clean generated gallery artifacts
gallery-clean:
	rm -f docs/gallery/*.gif docs/gallery/*.mp4

# --- Homebrew Tap helpers ---
# Generate a Formula/dt.rb using package/formula_template.rb and the current version.
# Optionally set TAP_DIR to your local tap repo (…/Library/Taps/<user>/homebrew-tap).

tap-formula:
	@bash package/gen_formula.sh --out package/Formula/dt.rb
	@echo "Formula generated at package/Formula/dt.rb"

# If TAP_DIR is provided and points to a homebrew-tap repo, copy the formula there.
tap-formula-to-tap: tap-formula
	@if [ -n "$$TAP_DIR" ] && [ -d "$$TAP_DIR" ]; then \
	  mkdir -p "$$TAP_DIR/Formula"; \
	  cp package/Formula/dt.rb "$$TAP_DIR/Formula/dt.rb"; \
	  echo "Copied formula to $$TAP_DIR/Formula/dt.rb"; \
	else \
	  echo "Set TAP_DIR to your local tap path to copy formula automatically."; \
	fi
