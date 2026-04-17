// Cron job management - scheduling, storage, and execution

use crate::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Cron jobs storage directory
pub fn cron_dir() -> PathBuf {
    Config::hermes_home().join("cron")
}

/// Cron jobs file path
pub fn cron_jobs_path() -> PathBuf {
    cron_dir().join("jobs.json")
}

/// Cron job output directory
pub fn cron_output_dir() -> PathBuf {
    cron_dir().join("output")
}

/// Schedule kind
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleKind {
    Once,
    Interval,
    Cron,
}

impl ScheduleKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "once" => Some(ScheduleKind::Once),
            "interval" => Some(ScheduleKind::Interval),
            "cron" => Some(ScheduleKind::Cron),
            _ => None,
        }
    }
}

/// Schedule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub kind: ScheduleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    pub display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_at: Option<String>,
}

/// Repeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatConfig {
    #[serde(rename = "times")]
    pub max_times: Option<u32>,
    pub completed: u32,
}

/// Cron job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub schedule: Schedule,
    pub schedule_display: String,
    pub repeat: RepeatConfig,
    pub enabled: bool,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_reason: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliver: Option<String>,
}

impl CronJob {
    pub fn new(id: &str, name: &str, prompt: &str, schedule: Schedule) -> Self {
        let display = schedule.display.clone();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            prompt: prompt.to_string(),
            skills: vec![],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            schedule,
            schedule_display: display,
            repeat: RepeatConfig { max_times: None, completed: 0 },
            enabled: true,
            state: "scheduled".to_string(),
            paused_at: None,
            paused_reason: None,
            created_at: Utc::now().to_rfc3339(),
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            deliver: None,
        }
    }
}

/// Jobs storage wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobsStorage {
    jobs: Vec<CronJob>,
    updated_at: String,
}

impl Default for JobsStorage {
    fn default() -> Self {
        Self {
            jobs: vec![],
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Parse duration string into minutes
pub fn parse_duration(s: &str) -> Result<u32> {
    let s = s.trim().to_lowercase();
    let re = regex_lite::Regex::new(r"^(\d+)\s*(m|min|mins|h|hr|hrs|d|day|days)$").unwrap();
    
    if let Some(caps) = re.captures(&s) {
        let value: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let unit = caps.get(2).unwrap().as_str();
        
        let multiplier = match unit.chars().next().unwrap() {
            'm' => 1,
            'h' => 60,
            'd' => 1440,
            _ => 1,
        };
        return Ok(value * multiplier);
    }
    
    anyhow::bail!("Invalid duration format: '{}'. Use format like '30m', '2h', '1d'", s);
}

/// Parse schedule string
pub fn parse_schedule(s: &str) -> Result<Schedule> {
    let s = s.trim();
    let s_lower = s.to_lowercase();
    
    // "every X" pattern - recurring interval
    if s_lower.starts_with("every ") {
        let duration_str = &s[6..].trim();
        let minutes = parse_duration(duration_str)?;
        return Ok(Schedule {
            kind: ScheduleKind::Interval,
            minutes: Some(minutes),
            expr: None,
            display: format!("every {}m", minutes),
            run_at: None,
        });
    }
    
    // Cron expression (5 fields)
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() >= 5 {
        // Basic cron validation
        return Ok(Schedule {
            kind: ScheduleKind::Cron,
            minutes: None,
            expr: Some(s.to_string()),
            display: s.to_string(),
            run_at: None,
        });
    }
    
    // Duration like "30m", "2h", "1d" - one-shot
    if let Ok(minutes) = parse_duration(s) {
        let run_at = (Utc::now() + Duration::minutes(minutes as i64)).to_rfc3339();
        return Ok(Schedule {
            kind: ScheduleKind::Once,
            minutes: None,
            expr: None,
            display: format!("once in {}m", minutes),
            run_at: Some(run_at),
        });
    }
    
    // ISO timestamp
    if s.contains('T') || s.starts_with(|c: char| c.is_ascii_digit()) {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(Schedule {
                kind: ScheduleKind::Once,
                minutes: None,
                expr: None,
                display: format!("once at {}", dt.format("%Y-%m-%d %H:%M")),
                run_at: Some(dt.to_rfc3339()),
            });
        }
    }
    
    anyhow::bail!(
        "Invalid schedule '{}'. Use:\n\
        - Duration: '30m', '2h', '1d' (one-shot)\n\
        - Interval: 'every 30m', 'every 2h' (recurring)\n\
        - Cron: '0 9 * * *' (cron expression)\n\
        - Timestamp: '2024-12-25T14:00:00Z' (one-shot)",
        s
    );
}

/// Compute next run time based on schedule
pub fn compute_next_run(schedule: &Schedule, last_run_at: Option<&str>) -> Option<String> {
    match schedule.kind {
        ScheduleKind::Once => schedule.run_at.clone(),
        ScheduleKind::Interval => {
            let minutes = schedule.minutes.unwrap_or(30);
            let base = if let Some(last) = last_run_at {
                DateTime::parse_from_rfc3339(last).ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else {
                Some(Utc::now())
            };
            base.map(|t| (t + Duration::minutes(minutes as i64)).to_rfc3339())
        }
        ScheduleKind::Cron => {
            // For cron, we'd need a cron parser library
            // For now, just return None (not implemented)
            None
        }
    }
}

/// Ensure cron directories exist
pub fn ensure_dirs() -> Result<()> {
    let dirs = [cron_dir(), cron_output_dir()];
    for dir in &dirs {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create cron directory {:?}", dir))?;
    }
    Ok(())
}

/// Load all jobs from storage
pub fn load_jobs() -> Result<Vec<CronJob>> {
    ensure_dirs()?;
    let path = cron_jobs_path();
    
    if !path.exists() {
        return Ok(vec![]);
    }
    
    let content = fs::read_to_string(&path)
        .context("failed to read cron jobs file")?;
    
    let storage: JobsStorage = serde_json::from_str(&content)
        .context("failed to parse cron jobs JSON")?;
    
    Ok(storage.jobs)
}

/// Save all jobs to storage
pub fn save_jobs(jobs: &[CronJob]) -> Result<()> {
    ensure_dirs()?;
    let path = cron_jobs_path();
    
    let storage = JobsStorage {
        jobs: jobs.to_vec(),
        updated_at: Utc::now().to_rfc3339(),
    };
    
    let content = serde_json::to_string_pretty(&storage)
        .context("failed to serialize cron jobs")?;
    
    fs::write(&path, content)
        .context("failed to write cron jobs file")?;
    
    Ok(())
}

/// Create a new cron job
pub fn create_job(prompt: String, schedule: String) -> Result<CronJob> {
    let parsed = parse_schedule(&schedule)?;
    let id = uuid_simple();
    let job = CronJob::new(&id, &prompt, &prompt, parsed);

    let mut jobs = load_jobs()?;
    jobs.push(job.clone());
    save_jobs(&jobs)?;

    Ok(job)
}

/// Get a job by ID
pub fn get_job(job_id: &str) -> Result<Option<CronJob>> {
    let jobs = load_jobs()?;
    Ok(jobs.into_iter().find(|j| j.id == job_id))
}

/// List all jobs
pub fn list_jobs(include_disabled: bool) -> Result<Vec<CronJob>> {
    let jobs = load_jobs()?;
    if include_disabled {
        return Ok(jobs);
    }
    Ok(jobs.into_iter().filter(|j| j.enabled).collect())
}

/// Remove a job by ID
pub fn remove_job(job_id: &str) -> Result<bool> {
    let mut jobs = load_jobs()?;
    let original_len = jobs.len();
    jobs.retain(|j| j.id != job_id);

    if jobs.len() < original_len {
        save_jobs(&jobs)?;
        return Ok(true);
    }
    Ok(false)
}

/// Pause a job
pub fn pause_job(job_id: &str, reason: Option<&str>) -> Result<Option<CronJob>> {
    let mut jobs = load_jobs()?;

    for job in &mut jobs {
        if job.id == job_id {
            job.enabled = false;
            job.state = "paused".to_string();
            job.paused_at = Some(Utc::now().to_rfc3339());
            job.paused_reason = reason.map(String::from);
            let updated = job.clone();
            save_jobs(&jobs)?;
            return Ok(Some(updated));
        }
    }

    Ok(None)
}

/// Resume a paused job
pub fn resume_job(job_id: &str) -> Result<Option<CronJob>> {
    let mut jobs = load_jobs()?;

    for job in &mut jobs {
        if job.id == job_id {
            job.enabled = true;
            job.state = "scheduled".to_string();
            job.paused_at = None;
            job.paused_reason = None;
            job.next_run_at = compute_next_run(&job.schedule, job.last_run_at.as_deref());
            let updated = job.clone();
            save_jobs(&jobs)?;
            return Ok(Some(updated));
        }
    }
    
    Ok(None)
}

/// Get all jobs that are due to run now
pub fn get_due_jobs() -> Vec<CronJob> {
    let jobs = match load_jobs() {
        Ok(j) => j,
        Err(_) => return vec![],
    };

    let now = chrono::Utc::now();
    let mut due = vec![];

    for job in jobs {
        if !job.enabled {
            continue;
        }
        if let Some(next_run) = &job.next_run_at {
            if let Ok(next_dt) = DateTime::parse_from_rfc3339(next_run) {
                if next_dt.with_timezone(&chrono::Utc) <= now {
                    due.push(job);
                }
            }
        }
    }

    due
}

/// Simple UUID generator
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30m").unwrap(), 30);
        assert_eq!(parse_duration("2h").unwrap(), 120);
        assert_eq!(parse_duration("1d").unwrap(), 1440);
    }

    #[test]
    fn test_parse_schedule() {
        let s = parse_schedule("30m").unwrap();
        assert_eq!(s.kind, ScheduleKind::Once);
        
        let s = parse_schedule("every 30m").unwrap();
        assert_eq!(s.kind, ScheduleKind::Interval);
        
        let s = parse_schedule("every 2h").unwrap();
        assert_eq!(s.kind, ScheduleKind::Interval);
        assert_eq!(s.minutes, Some(120));
    }

    #[test]
    fn test_uuid_simple() {
        let id1 = uuid_simple();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = uuid_simple();
        assert_eq!(id1.len(), 12);
        assert_eq!(id2.len(), 12);
        // IDs should be 12 hex chars
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}