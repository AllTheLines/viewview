function danger_reset_atlas {
	rm output/archive/* || true
	rm website/public/longest_lines/* || true
	rm output/status.json || true
}
