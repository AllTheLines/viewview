R2_HOST=364f498af774cadc655afe7f4ef9b8b5.r2.cloudflarestorage.com

function s3 {
	_uvx s3cmd \
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

function rclone_put {
	local source=$1
	local destination=$2

	rclone copy "$source" :s3:"$destination" \
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

function get_srtm_folder {
	local destination=$1

	get_s3_folder SRTM "$destination"
}

function get_s3_folder {
	local source=$1
	local destination=$2

	rclone copy :s3:/"$source" "$destination" \
		--s3-endpoint=https://$R2_HOST \
		--s3-provider=Cloudflare \
		--s3-access-key-id="$VIEWVIEW_S3_ACCESS_KEY" \
		--s3-secret-access-key="$VIEWVIEW_S3_SECRET" \
		--progress \
		--transfers=16 \
		--checkers=16 \
		--multi-thread-streams=16
}

function download_all_tvs_tiffs {
	local version=$1

	get_s3_folder viewview/runs/"$version"/tvs output/archive
}

function make_prod_pmtiles {
	local version=$1

	make_pmtiles "$version" website/public/world.pmtiles
}
