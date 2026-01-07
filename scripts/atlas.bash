ATLAS_DB=state/atlas.db

function sqlite3 {
	command sqlite3 -cmd ".headers on" -cmd ".mode column" "$@"
}

function atlas_query {
	local query=$1

	sqlite3 "$ATLAS_DB" "$query"
}

function remove_machines_older_than {
	local last_seen=$1

	if [ -z "$last_seen" ]; then
		list_machines
	else
		atlas_query "
		  DELETE FROM Jobs
			  WHERE
		      job_type = 'tasks::atlas::machines::new_machine_job::NewMachineJob'
		    AND run_at <= $last_seen
		"
		atlas_query "
		  DELETE FROM Workers
			  WHERE
		      worker_type = 'tasks::atlas::tile_job::TileJob'
		    AND last_seen <= $last_seen
		"
		list_machines
	fi

}

function list_machines {
	echo "All machines: "
	atlas_query "
		SELECT
			job,
			run_at,
			datetime(run_at, 'unixepoch', 'localtime') as datetime
		FROM Jobs
			WHERE
				job_type = 'tasks::atlas::machines::new_machine_job::NewMachineJob'
			AND
			  status = 'Done'
	"
}

function restart_job {
	local job=$1

	atlas_query "
	  UPDATE Jobs SET
		  status = 'Pending',
			run_at = strftime('%s', 'now'),
      lock_at = NULL,
      lock_by = NULL,
		  done_at = NULL,
      last_result = NULL,
			attempts = 0
    WHERE id = '$job'
	"
}

function restart_failed_jobs {
	local job=$1

	atlas_query "
	  UPDATE Jobs SET
		  status = 'Pending',
			run_at = strftime('%s', 'now'),
      lock_at = NULL,
      lock_by = NULL,
		  done_at = NULL,
      last_result = NULL,
			attempts = 0
    WHERE status='Failed'
	"
}
