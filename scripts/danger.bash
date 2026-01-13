function danger_reset_atlas {
	rm state/atlas* || true
	rm output/archive/* || true
	rm website/public/longest_lines/* || true
}

function danger_delete_all_tile_jobs {
	atlas_query "
	  DELETE FROM Jobs
		WHERE job_type = 'tasks::atlas::tile_job::TileJob'
	"
}
