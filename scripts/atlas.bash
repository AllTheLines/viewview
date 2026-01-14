ATLAS_DB=state/atlas.db

function sqlite3 {
	command sqlite3 -cmd ".headers on" -cmd ".mode column" "$@"
}

function atlas_query {
	local query=$1

	sqlite3 "$ATLAS_DB" "$query"
}

# Get the current run ID via the latest successful tile job in the DB.
function get_current_run_id {
	json=$(RUST_LOG=off cargo run --bin tasks -- atlas current-run-config)
	echo "$json" | jq --raw-output '.run_id'
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
		  id,
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
	atlas_query "
	  UPDATE Jobs SET
		  status = 'Pending',
			run_at = strftime('%s', 'now'),
      lock_at = NULL,
      lock_by = NULL,
		  done_at = NULL,
      last_result = NULL,
			attempts = 0
    WHERE status='Killed' OR status='Failed'
	"
}

function rebalance_queue {
	atlas_query "
	  UPDATE Jobs SET
		  status = 'Pending',
			run_at = strftime('%s', 'now'),
      lock_at = NULL,
      lock_by = NULL,
		  done_at = NULL,
      last_result = NULL,
			attempts = 0
    WHERE status='Queued'
	"
}
