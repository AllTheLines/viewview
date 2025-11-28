#!/bin/bash

export PROJECT_ROOT
PROJECT_ROOT=$(dirname "$(readlink -f "$0")")

export GDAL_NUM_THREADS=ALL_CPUS

function _load_includes {
	for file in "$PROJECT_ROOT"/scripts/*.bash; do
		# shellcheck disable=1090
		source "$file"
	done
}

_load_includes
_load_env

subcommand=$1
shift
args=("$@")
"$subcommand" "${args[@]}"
