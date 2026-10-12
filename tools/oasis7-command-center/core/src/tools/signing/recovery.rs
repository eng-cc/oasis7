use super::*;

/// Reconcile only the original submitted identity. Missing/reserved observations do
/// not prove no eventual effect and deliberately retain the unknown reservation.
pub fn reconcile_signer_job(jobs: &jobs::ToolJobs, job_id: &str) -> Result<serde_json::Value> {
    let (resource, fingerprint) = frozen_job_identity(job_id, false)?;
    let (cfg, _) = binding()?;
    let directory = oasis7_local_signer::job::JobDirectory::open(
        &cfg,
        installation::current_uid(),
        job_id,
        false,
    )
    .map_err(invalid)?;
    let prepared = read_prepared(&directory)?;
    let result = signer_action("inspect", Some(job_id))?;
    let response: protocol::IpcResponse = serde_json::from_value(result.clone())?;
    if let protocol::IpcResponse::Inspect(response) = response {
        if response.request_id != prepared.request.request_id {
            return Err(invalid("Inspection changed original request identity"));
        }
        if response.status == "committed" && response.error_code.0.is_none() {
            let expected_operation = identity::operation_key(
                &cfg.installation_id,
                &cfg.deployment_id,
                &prepared.request.context.network_id,
                &prepared.request.purpose,
                &prepared.request.signer_id,
                &identity::sha256(&prepared.payload),
            )
            .map_err(invalid)?;
            if response.operation_key.0.as_deref() != Some(&expected_operation) {
                return Err(invalid(
                    "Committed operation key differs from original envelope",
                ));
            }
            if response.payload_sha256.0.as_deref()
                != Some(&identity::sha256_hex(&prepared.payload))
                || response.audit_digest.0.is_none()
            {
                return Err(invalid(
                    "Committed observation does not bind original payload/audit",
                ));
            }
            for job in jobs.list()? {
                if job.resource == resource && job.kind == "submit" && job.state == "unknown" {
                    if job.fingerprint != fingerprint {
                        return Err(invalid(
                            "Original submitted fingerprint differs; retain unknown",
                        ));
                    }
                    jobs.finish(
                        &job.id,
                        "succeeded",
                        Some("Original request committed; verified inspect receipt"),
                    )?;
                }
            }
        }
    } else {
        return Err(invalid("Expected original request inspection"));
    }
    Ok(result)
}

pub fn reconcile_file_prepare(
    jobs: &jobs::ToolJobs,
    input: &FileSignInput,
) -> Result<serde_json::Value> {
    let (resource, fingerprint) = file_prepare_identity(input)?;
    let (cfg, _) = binding()?;
    let job = oasis7_local_signer::job::JobDirectory::open(
        &cfg,
        installation::current_uid(),
        &input.job_id,
        false,
    )
    .map_err(invalid)?;
    let prepared = read_prepared(&job)?;
    let source_evidence = read_source_evidence(&job, &prepared, true)?;
    if source_evidence.file_path != input.file_path {
        return Err(invalid(
            "Original selected file path differs from preparation evidence",
        ));
    }
    let context: protocol::SignContext = serde_json::from_value(input.context.clone())?;
    let (digest, size) = file_digest(Path::new(&input.file_path))?;
    if prepared.request.installation_id != cfg.installation_id
        || prepared.request.context.deployment_id != cfg.deployment_id
        || prepared.request.context != context
        || prepared.request.signer_id != input.signer_id
        || prepared.request.grant_id.0.as_deref() != Some(&input.grant_id)
        || prepared.payload
            != oasis7_local_signer::detached::signing_payload_from_digest(&context, &digest, size)
                .map_err(invalid)?
    {
        return Err(invalid("Prepared files do not match original intent"));
    }
    for job in jobs.list()? {
        if job.resource == resource && job.kind == "file_prepare" && job.state == "unknown" {
            if job.fingerprint != fingerprint {
                return Err(invalid("Original preparation fingerprint differs"));
            }
            jobs.finish(
                &job.id,
                "succeeded",
                Some("Original prepared request files verified"),
            )?;
        }
    }
    Ok(
        serde_json::json!({"status":"PREPARED","job_id":input.job_id,"request_id":prepared.request.request_id,"payload_sha256":identity::sha256_hex(&prepared.payload),"signing_enabled":false}),
    )
}
