{{/*
Expand the name of the chart.
*/}}
{{- define "kotoba.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "kotoba.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "kotoba.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "kotoba.labels" -}}
helm.sh/chart: {{ include "kotoba.chart" . }}
{{ include "kotoba.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "kotoba.selectorLabels" -}}
app.kubernetes.io/name: {{ include "kotoba.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "kotoba.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "kotoba.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Generate node addresses for cluster configuration
*/}}
{{- define "kotoba.nodeAddress" -}}
{{- printf "%s-%d.%s.%s.svc.cluster.local:8080" (include "kotoba.fullname" .) .index (include "kotoba.fullname" .) .Release.Namespace }}
{{- end }}

{{/*
Generate cluster configuration
*/}}
{{- define "kotoba.clusterConfig" -}}
{
  metadata: {
    name: "{{ include "kotoba.fullname" . }}",
    version: "{{ .Chart.AppVersion }}",
    description: "Kotoba distributed storage cluster deployed with Helm",
  },

  cluster: {
    nodes: [
{{- range $i, $node := .Values.cluster.nodes }}
      {
        id: "{{ $node.id }}",
        address: "{{ tpl $node.address $ }}",
        role: "storage",
        cid_ranges: [
          { start: {{ $node.cidRange.start }}, end: {{ $node.cidRange.end }} }
        ],
      },
{{- end }}
    ],
    replication_factor: {{ .Values.cluster.replicationFactor }},
    read_consistency: "{{ .Values.cluster.readConsistency }}",
    write_consistency: "{{ .Values.cluster.writeConsistency }}",
  },

  storage: {
    data_dir: "/data",
    memtable_size: 1000,
    sstable_max_size: 1073741824,
    compaction_interval: 3600,
    snapshot_interval: 86400,
  },

  server: {
    http_port: 3000,
    grpc_port: 8080,
    host: "0.0.0.0",
    max_connections: 1000,
  },

  monitoring: {
    metrics_enabled: {{ .Values.monitoring.enabled }},
    health_check_enabled: {{ .Values.healthCheck.enabled }},
    prometheus_endpoint: "{{ .Values.monitoring.metricsPath }}",
    health_endpoint: "{{ .Values.healthCheck.path }}",
  },

  environment: {
    NODE_ENV: "production",
    RUST_LOG: "info,kotoba=debug",
    K8S_DEPLOYMENT: "true",
  },
}
{{- end }}
