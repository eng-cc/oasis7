use super::*;

pub(super) fn prepared_identity(
    job_id: &str,
    cfg: &oasis7_local_signer::types::InstallationConfig,
    prepared: &oasis7_local_signer::job::PreparedJob,
    source: &FileSourceEvidence,
) -> Result<(String, String)> {
    if prepared.request.installation_id != cfg.installation_id
        || prepared.request.context.deployment_id != cfg.deployment_id
    {
        return Err(invalid("Prepared request belongs to another installation"));
    }
    let fingerprint = identity::sha256_hex(
        &identity::canonical_json(&(
            cfg.installation_id.clone(),
            &cfg.deployment_id,
            &cfg.release_id,
            &prepared.request,
            identity::sha256_hex(&prepared.payload),
            source,
        ))
        .map_err(invalid)?,
    );
    Ok((
        format!("signer:{}:{}", cfg.installation_id, job_id),
        fingerprint,
    ))
}

pub fn submit_file_frozen(job_id: &str, expected_fingerprint: &str) -> Result<serde_json::Value> {
    let (cfg, executable) = binding()?;
    let job = oasis7_local_signer::job::JobDirectory::open(
        &cfg,
        installation::current_uid(),
        job_id,
        false,
    )
    .map_err(invalid)?;
    let prepared = read_prepared(&job)?;
    let source = read_source_evidence(&job, &prepared, true)?;
    if prepared_identity(job_id, &cfg, &prepared, &source)?.1 != expected_fingerprint {
        return Err(invalid("Original previewed signing request changed"));
    }
    let expected_request = prepared.submission_identity(&cfg).map_err(invalid)?;
    let binary = executable
        .to_str()
        .ok_or_else(|| invalid("Caller path must be UTF-8"))?;
    let info: serde_json::Value = serde_json::from_slice(&crate::process::run(
        binary,
        &["info", "--json"],
        Path::new("/"),
    )?)?;
    if !compatible_info(&info)
        || !info["capabilities"].as_array().is_some_and(|capabilities| {
            capabilities
                .iter()
                .any(|value| value["id"] == "frozen_submit_v1" && value["supported"] == true)
        })
    {
        return Err(invalid(
            "Installed caller lacks frozen_submit_v1; upgrade the caller before submitting",
        ));
    }
    Ok(serde_json::from_slice(&crate::process::run(
        binary,
        &[
            "submit",
            "--job-id",
            job_id,
            "--expected-request-sha256",
            &expected_request,
        ],
        Path::new("/"),
    )?)?)
}
