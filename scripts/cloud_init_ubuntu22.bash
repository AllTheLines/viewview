function cloud_init_ubuntu22 {
	set -euo pipefail

	local address=$1

	ssh -o StrictHostKeyChecking=accept-new "$address" "\
		set -euo pipefail
	  # On Digital Ocean, machines can startup with an existing apt process, so we need
		# to wait for its lock to be released before updating.
		while lsof /var/lib/apt/lists/lock &>/dev/null; do sleep 2; done;
	  sudo apt update
		sudo apt install --yes \
			libvulkan1 mesa-vulkan-drivers vulkan-tools \
			build-essential pkg-config \
			libgdal-dev gdal-bin python3-gdal rsync htop \
			jq rclone tmux sqlite3
	  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
		echo 'source ~/.cargo/env' >> ~/.bashrc
		curl -LsSf https://astral.sh/uv/install.sh | sh
		echo 'source ~/.local/bin/env' >> ~/.bashrc
	"

	_pushd "$PROJECT_ROOT/../total-viewsheds/"
	git ls-files -z | rsync -avP --files-from=- --from0 ./ "$address":tvs

	_pushd "$PROJECT_ROOT/"
	rsync -avP ./ctl.sh "$address":viewview/
	rsync -avP ./.env "$address":viewview/
	rsync -avP scripts "$address":viewview/

	ssh "$address" "\
		set -euo pipefail
	  source ~/.cargo/env
	  cd ~/tvs && cargo build --release
	"
}
