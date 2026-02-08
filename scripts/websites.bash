function webapp_deploy {
	_pushd "$PROJECT_ROOT"/website
	pnpm run build
	npx wrangler deploy
	_popd
}

function static_site_deploy {
	_pushd "$PROJECT_ROOT"/ssg
	hugo
	npx wrangler deploy
	_popd
}
