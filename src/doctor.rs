use anyhow::Result;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub status: String,
    pub cwd: String,
    pub executable: String,
    pub checks: Vec<DoctorCheck>,
}

pub fn doctor_environment() -> Result<DoctorReport> {
    let cwd = env::current_dir()?;
    let executable = env::current_exe()?;
    let temp_probe = env::temp_dir().join("zeropdf-doctor-probe.tmp");
    let state_dir = Path::new(".zeropdf");

    let temp_check = match fs::write(&temp_probe, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&temp_probe);
            DoctorCheck {
                name: "temp_dir_write".to_string(),
                ok: true,
                detail: env::temp_dir().display().to_string(),
            }
        }
        Err(err) => DoctorCheck {
            name: "temp_dir_write".to_string(),
            ok: false,
            detail: err.to_string(),
        },
    };

    let state_check = match fs::create_dir_all(state_dir) {
        Ok(_) => DoctorCheck {
            name: "state_dir_ready".to_string(),
            ok: true,
            detail: state_dir.display().to_string(),
        },
        Err(err) => DoctorCheck {
            name: "state_dir_ready".to_string(),
            ok: false,
            detail: err.to_string(),
        },
    };

    Ok(DoctorReport {
        status: "success".to_string(),
        cwd: cwd.display().to_string(),
        executable: executable.display().to_string(),
        checks: vec![
            DoctorCheck {
                name: "cwd_exists".to_string(),
                ok: cwd.exists(),
                detail: cwd.display().to_string(),
            },
            DoctorCheck {
                name: "binary_exists".to_string(),
                ok: executable.exists(),
                detail: executable.display().to_string(),
            },
            temp_check,
            state_check,
        ],
    })
}
