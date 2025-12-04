R2_HOST=364f498af774cadc655afe7f4ef9b8b5.r2.cloudflarestorage.com

function s3 {
	/root/.local/bin/uvx s3cmd \
		--host https://$R2_HOST \
		--host-bucket="%(bucket)s.$R2_HOST" \
		--access_key="$VIEWVIEW_S3_ACCESS_KEY" \
		--secret_key="$VIEWVIEW_S3_SECRET" \
		"$@"
}

function s3fs_mount {
	local bucket=$1
	local mountpoint=$2
	export AWS_ACCESS_KEY_ID="$VIEWVIEW_S3_ACCESS_KEY"
	export AWS_SECRET_ACCESS_KEY="$VIEWVIEW_S3_SECRET"

	s3fs "$bucket" "$mountpoint" -o url=https://$R2_HOST
}

function put_srtm_folder {
	local source=$1

	rclone copy "$source" :s3:viewview/SRTM \
		--include "*.hgt" \
		--s3-endpoint=https://$R2_HOST \
		--s3-provider=Cloudflare \
		--s3-access-key-id="$VIEWVIEW_S3_ACCESS_KEY" \
		--s3-secret-access-key="$VIEWVIEW_S3_SECRET" \
		--progress \
		--transfers=32 \
		--checkers=32 \
		--multi-thread-streams=16 \
		--multi-thread-cutoff=0 \
		--s3-chunk-size=64M \
		--s3-upload-concurrency=16
}
