function danger_reset_atlas {
	rm state/atlas* || true
	rm output/archive/* || true
	rm website/public/longest_lines/* || true
}
