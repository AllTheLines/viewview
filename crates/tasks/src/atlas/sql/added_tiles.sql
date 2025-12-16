SELECT
  CAST(job AS TEXT) as tile
FROM Jobs
WHERE job_type = 'tasks::atlas::tile_job::TileJob'
ORDER BY done_at DESC;
