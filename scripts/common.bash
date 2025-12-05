# Where to save output
export OUTPUT_DIR=output
# Where to keep previous COGs so that aggregated `.pmtile`s can be rebuilt.
export ARCHIVE_DIR=$OUTPUT_DIR/archive
# Temp space
export TMP_DIR=$OUTPUT_DIR/tmp

# shellcheck disable=2120
function _panic() {
	local message=$1
	echo >&2 "$message"
	exit 1
}

function _pushd {
	# shellcheck disable=2119
	command pushd "$@" >/dev/null || _panic
}

function _popd {
	# shellcheck disable=2119
	command popd "$@" >/dev/null || _panic
}

function ensure_tiles_env {
	if [ -z "${OUTPUT_DIR:-}" ]; then
		echo "OUTPUT_DIR is not set"
		exit 1
	fi

	if [ -z "${ARCHIVE_DIR:-}" ]; then
		echo "ARCHIVE_DIR is not set"
		exit 1
	fi

	if [ -z "${TMP_DIR:-}" ]; then
		echo "TMP_DIR is not set"
		exit 1
	fi
}

function _load_env {
	set -a
	source "$PROJECT_ROOT"/.env
	set +a
}

function _uvx {
	if type -P uvx >/dev/null 2>&1; then
		uvx "$@"
	else
		# Likely what's need on a remote machine over a non-interactive SSH connection.
		/root/.local/bin/uvx "$@"
	fi
}
