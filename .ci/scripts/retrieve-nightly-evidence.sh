#!/usr/bin/env bash
set -euo pipefail

# Run with a Kubernetes context allowed to create/delete an ephemeral Pod in
# argo-workflows. The Pod receives the existing read-only Garage key by Secret
# reference; credential bytes never enter the local shell or command line.
if [[ $# -ne 3 ]]; then
    echo "usage: $0 <pdftract-nightly-fuzz|pdftract-nightly-supply-chain> <workflow-name> <output.tgz>" >&2
    exit 2
fi
schedule=$1
workflow=$2
output=$3
case "$schedule" in
    pdftract-nightly-fuzz|pdftract-nightly-supply-chain) ;;
    *) echo "invalid schedule" >&2; exit 2 ;;
esac
if [[ ! "$workflow" =~ ^${schedule}-[a-z0-9-]+$ ]]; then
    echo "workflow name does not match schedule" >&2
    exit 2
fi
if [[ -e "$output" ]]; then
    echo "output already exists: $output" >&2
    exit 2
fi

pod=''
cleanup() {
    if [[ -n "$pod" ]]; then
        kubectl -n argo-workflows delete pod "$pod" --wait=false >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

pod=$(kubectl -n argo-workflows create -f - -o jsonpath='{.metadata.name}' <<'YAML'
apiVersion: v1
kind: Pod
metadata:
  generateName: pdftract-evidence-reader-
  labels:
    app.kubernetes.io/name: pdftract-evidence-reader
spec:
  restartPolicy: Never
  containers:
    - name: reader
      image: docker.io/amazon/aws-cli:2.36.44
      command: [sh, -c, "sleep 3600"]
      env:
        - name: AWS_ACCESS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: needle-ci-local-artifact-reader
              key: access-key
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: needle-ci-local-artifact-reader
              key: secret-key
        - name: AWS_DEFAULT_REGION
          value: garage
      resources:
        requests:
          cpu: 50m
          memory: 64Mi
        limits:
          cpu: 250m
          memory: 256Mi
YAML
)
kubectl -n argo-workflows wait --for=condition=Ready "pod/$pod" --timeout=120s >/dev/null

key="pdftract/nightly/$schedule/$workflow/evidence.tgz"
endpoint=http://needle-ci-artifacts-iad.argo-workflows.svc.cluster.local:3900
kubectl -n argo-workflows exec "$pod" -c reader -- \
    aws --endpoint-url="$endpoint" s3api head-object \
    --bucket needle-ci-artifacts --key "$key" >/dev/null
kubectl -n argo-workflows exec "$pod" -c reader -- \
    aws --endpoint-url="$endpoint" s3 cp --no-progress \
    "s3://needle-ci-artifacts/$key" /tmp/evidence.tgz >/dev/null
if ! kubectl -n argo-workflows exec "$pod" -c reader -- cat /tmp/evidence.tgz > "$output"; then
    rm -f -- "$output"
    exit 1
fi
tar -tzf "$output" >/dev/null
echo "$output"
