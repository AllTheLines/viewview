function init_digital_ocean {
	set -euo pipefail

	local ip_address=$1

	ssh -o StrictHostKeyChecking=accept-new root@"$ip_address" "\
		set -euo pipefail
		apt install --yes \
			libvulkan1 mesa-vulkan-drivers build-essential pkg-config \
			libgdal-dev gdal-bin python3-gdal jq
	  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
		echo 'source ~/.cargo/env' >> ~/.bashrc
		curl -LsSf https://astral.sh/uv/install.sh | sh
		echo 'source ~/.local/bin/env' >> ~/.bashrc
	  mkdir -p ~/tvs/output
	  mkdir -p ~/viewview/output/archive
	"

	_pushd "$PROJECT_ROOT/../total-viewsheds/"
	git ls-files -z | rsync -avP --files-from=- --from0 ./ root@"$ip_address":tvs

	_pushd "$PROJECT_ROOT/"
	rsync -avP ./ctl.sh root@"$ip_address":viewview/
	rsync -avP ./.env root@"$ip_address":viewview/
	rsync -avP scripts root@"$ip_address":viewview/

	ssh root@"$ip_address" "\
		set -euo pipefail
	  source ~/.cargo/env
	  cd ~/tvs && cargo build --release
	"
}
