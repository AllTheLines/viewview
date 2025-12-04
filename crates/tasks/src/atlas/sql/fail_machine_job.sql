UPDATE Jobs
SET status='Failed',
    last_result=$1,
    lock_by=NULL
WHERE id = $2;
