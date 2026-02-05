


function spin_google_cloud {
  gcloud compute instances create "$1" --format="json" \
      --zone=us-central1-a \
      --machine-type=c4d-highcpu-96 \
      --network-interface=network-tier=PREMIUM,nic-type=GVNIC,stack-type=IPV4_ONLY,subnet=default \
      --metadata=startup-script="sudo rm -r /usr/lib/google-cloud-sdk/",ssh-keys=atlas:"$2" \
      --no-restart-on-failure \
      --maintenance-policy=TERMINATE \
      --provisioning-model=SPOT \
      --instance-termination-action=STOP \
      --scopes=https://www.googleapis.com/auth/devstorage.read_only,https://www.googleapis.com/auth/logging.write,https://www.googleapis.com/auth/monitoring.write,https://www.googleapis.com/auth/service.management.readonly,https://www.googleapis.com/auth/servicecontrol,https://www.googleapis.com/auth/trace.append \
      --min-cpu-platform=AMD\ \
Turin \
      --create-disk=auto-delete=yes,boot=yes,device-name="$1",disk-resource-policy=projects/viewshed-474722/regions/us-central1/resourcePolicies/default-schedule-1,image=projects/debian-cloud/global/images/debian-12-bookworm-v20260114,mode=rw,provisioned-iops=3060,provisioned-throughput=155,size=10,type=hyperdisk-balanced \
      --no-shielded-secure-boot \
      --shielded-vtpm \
      --shielded-integrity-monitoring \
      --labels=goog-ec-src=vm_add-gcloud \
      --reservation-affinity=none \
      --threads-per-core=1 \
      --format='value(networkInterfaces[0].accessConfigs[0].natIP)'
}

