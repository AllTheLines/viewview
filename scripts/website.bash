function website_deploy {
	_pushd "$PROJECT_ROOT"/website
	pnpm run build
	npx wrangler deploy
	_popd
}
