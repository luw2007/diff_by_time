.PHONY: gallery gallery-clean gallery-prep

# Render all VHS tapes under docs/vhs into GIFs under docs/gallery
gallery-prep:
	./scripts/demo_prep.sh all

gallery: gallery-prep
	@set -e; \
	for f in docs/vhs/*.tape; do \
	  echo "[VHS] Rendering $$f"; \
	  vhs "$$f"; \
	done

# Clean generated gallery artifacts
gallery-clean:
	rm -f docs/gallery/*.gif docs/gallery/*.mp4
