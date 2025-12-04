SELECT
  id,
  CAST(job AS TEXT) as machine
FROM Jobs
WHERE job_type = 'tasks::atlas::machines::new_machine_job::NewMachineJob'
  AND status = 'Done'
ORDER BY done_at DESC;
