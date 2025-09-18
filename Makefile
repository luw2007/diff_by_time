.PHONY: gallery gallery-clean

# Render all VHS tapes under docs/vhs into GIFs under docs/gallery
gallery:
	vhs docs/vhs/*.tape

# Clean generated gallery artifacts
gallery-clean:
	rm -f docs/gallery/*.gif docs/gallery/*.mp4

