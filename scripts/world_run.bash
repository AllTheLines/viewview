
function provision_gcloud {
    cargo run --bin tasks atlas new-machine --provider google-cloud --ssh-key-id "$(cat ~/.ssh/id_rsa.pub)"
}

function run_worker {
    eval "$(ssh-agent)"
    ssh-add ~/.ssh/id_rsa
    RUST_LOG=info,axum=trace,apalis=trace,tasks=trace cargo run --release --bin tasks -- atlas worker
}

function world_run {

  provision_gcloud &
  provision_gcloud &
  provision_gcloud &
  provision_gcloud &

  RUST_LOG=off,tasks=trace cargo run --bin tasks -- atlas run \
    --provider google-cloud \
    --run-id "$1" \
    --master website/public/tiles.csv \
    --centre -98.31396023805733,53.51619918320168 \
    --enable-cleanup \
    --tvs-executable /home/atlas/tvs/target/release/total-viewsheds \
    --longest-lines-cogs output/longest_lines

  wait
}