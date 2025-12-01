SELECT
  CAST(job AS TEXT) as tile
FROM Jobs
WHERE job_type = 'tasks::atlas::tile_job::TileJob'
  AND status = 'Done'
ORDER BY done_at DESC;

