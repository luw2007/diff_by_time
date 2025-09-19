.PHONY: gallery gallery-clean

# Render all VHS tapes under docs/vhs into GIFs under docs/gallery
gallery:
	@set -e; \
	for f in docs/vhs/*.tape; do \
	  echo "[VHS] Rendering $$f"; \
	  vhs "$$f"; \
	done

# Clean generated gallery artifacts
gallery-clean:
	rm -f docs/gallery/*.gif docs/gallery/*.mp4
