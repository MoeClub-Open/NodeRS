//! Final Xboard reports cannot be retried after an ambiguous response because
//! the source API has no idempotency key. Keep those reports for reconciliation.
use super::{MachinePanelClient, NodePanelClient, TrafficReportError};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs, io::AsyncWriteExt};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct RecoveryRecord {
    panel: String,
    machine_id: i64,
    node_id: i64,
    traffic: HashMap<i64, [u64; 2]>,
    status: String,
}

impl RecoveryRecord {
    fn matches_machine(&self, machine: &MachinePanelClient) -> bool {
        self.panel == machine.base_url && self.machine_id == machine.machine_id
    }
}

impl NodePanelClient {
    pub async fn preserve_uncertain_traffic(
        &self,
        traffic: &HashMap<i64, [u64; 2]>,
    ) -> anyhow::Result<()> {
        if self.machine.pxboard.is_some() || traffic.is_empty() {
            return Ok(());
        }
        let path = record_path("xboard-unconfirmed", self.node_id)?;
        write_record(&path, &self.recovery_record(traffic.clone(), "unconfirmed")).await?;
        tracing::warn!(
            path = %path.display(),
            "Xboard traffic delivery is unconfirmed; saved for manual reconciliation"
        );
        Ok(())
    }

    pub async fn replay_final_traffic(&self) -> anyhow::Result<()> {
        if self.machine.pxboard.is_some() {
            return Ok(());
        }
        let records = read_records().await?;
        let matching: Vec<_> = records
            .into_iter()
            .filter(|(_, record)| {
                record.matches_machine(&self.machine)
                    && record.node_id == self.node_id
                    && record.status == "pending"
            })
            .collect();
        for (path, mut record) in matching {
            // Mark before sending, so interruption during the request is never
            // mistaken for proof that the source did not receive the report.
            record.status = "unconfirmed".into();
            write_record(&path, &record).await?;
            match self.send_xboard_traffic(record.traffic.clone()).await {
                Ok(()) => {
                    fs::remove_file(&path).await?;
                    sync_directory(path.parent().unwrap_or(Path::new("."))).await?;
                }
                Err(TrafficReportError::Definite(_)) => {
                    record.status = "pending".into();
                    write_record(&path, &record).await?;
                    bail!(
                        "Xboard final traffic was not delivered; retained for retry: {}",
                        path.display()
                    );
                }
                Err(TrafficReportError::Uncertain(_)) => {
                    bail!(
                        "Xboard final traffic delivery is unconfirmed; reconcile: {}",
                        path.display()
                    );
                }
            }
        }
        Ok(())
    }

    pub async fn report_final_traffic(
        &self,
        traffic: HashMap<i64, [u64; 2]>,
    ) -> Result<(), TrafficReportError> {
        if let Some(panel) = &self.machine.pxboard {
            return panel.traffic(self.node_id, traffic).await;
        }
        if !traffic.is_empty() {
            let path =
                record_path("xboard-final", self.node_id).map_err(TrafficReportError::Definite)?;
            if let Err(error) = write_record(&path, &self.recovery_record(traffic, "pending")).await
            {
                // A directory sync can fail after the record was installed.
                // Its counters already belong to that record in this case.
                return Err(if fs::try_exists(&path).await.unwrap_or(true) {
                    TrafficReportError::Uncertain(error)
                } else {
                    TrafficReportError::Definite(error)
                });
            }
        }
        // Once durable, the caller must not restore the same counters. The
        // record owns retry or reconciliation, including definite send errors.
        self.replay_final_traffic()
            .await
            .map_err(TrafficReportError::Uncertain)
    }

    fn recovery_record(&self, traffic: HashMap<i64, [u64; 2]>, status: &str) -> RecoveryRecord {
        RecoveryRecord {
            panel: self.machine.base_url.clone(),
            machine_id: self.machine.machine_id,
            node_id: self.node_id,
            traffic,
            status: status.into(),
        }
    }
}

impl MachinePanelClient {
    pub async fn ensure_traffic_reconciled(&self) -> anyhow::Result<()> {
        if let Some(panel) = &self.pxboard {
            return panel.drain_outboxes().await;
        }
        let records = read_records().await?;
        let mut pending_nodes = HashSet::new();
        for (path, record) in records {
            if !record.matches_machine(self) || record.status == "resolved" {
                continue;
            }
            if record.status != "pending" {
                bail!(
                    "Xboard traffic must be reconciled before panel migration: {}",
                    path.display()
                );
            }
            pending_nodes.insert(record.node_id);
        }
        // Include removed nodes: their final report still belongs to the source.
        for node_id in pending_nodes {
            self.node_client(node_id).replay_final_traffic().await?;
        }
        Ok(())
    }
}

fn record_path(prefix: &str, node_id: i64) -> anyhow::Result<PathBuf> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::current_dir()?.join(format!(
        "{prefix}-node-{node_id}-{nonce}-{}-{}.json",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )))
}

async fn read_records() -> anyhow::Result<Vec<(PathBuf, RecoveryRecord)>> {
    let mut entries = fs::read_dir(std::env::current_dir()?).await?;
    let mut records = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".json")
            || !(name.starts_with("xboard-final-") || name.starts_with("xboard-unconfirmed-"))
        {
            continue;
        }
        let path = entry.path();
        let raw = match fs::read(&path).await {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let record = serde_json::from_slice(&raw)
            .with_context(|| format!("read saved Xboard traffic from {}", path.display()))?;
        records.push((path, record));
    }
    records.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(records)
}

async fn write_record(path: &Path, record: &RecoveryRecord) -> anyhow::Result<()> {
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary).await?;
    file.write_all(&serde_json::to_vec(record)?).await?;
    file.sync_all().await?;
    drop(file);
    fs::rename(&temporary, path).await?;
    sync_directory(path.parent().unwrap_or(Path::new("."))).await
}

async fn sync_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    fs::File::open(path).await?.sync_all().await?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
