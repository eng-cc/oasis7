use aliyun_tablestore_rs::{
    data::{GetRowRequest, PutRowRequest, UpdateRowRequest},
    error::OtsError,
    model::{ColumnValue, Row},
    protos::{ReturnType, RowExistenceExpectation},
    table::CreateTableRequest,
    OtsClient,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::future::Future;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tokio::runtime::{Builder, Runtime};

const HOSTED_ACCOUNT_STORE_BACKEND_ENV: &str = "OASIS7_HOSTED_ACCOUNT_STORE_BACKEND";
const HOSTED_ACCOUNT_STORE_BACKEND_AUTO: &str = "auto";
const HOSTED_ACCOUNT_STORE_BACKEND_FILE: &str = "file";
const HOSTED_ACCOUNT_STORE_BACKEND_TABLESTORE: &str = "tablestore";
const HOSTED_ACCOUNT_STORE_PATH_ENV: &str = "OASIS7_HOSTED_ACCOUNT_STORE_PATH";
const HOSTED_ACCOUNT_TABLESTORE_ENDPOINT_ENV: &str = "OASIS7_HOSTED_ACCOUNT_TABLESTORE_ENDPOINT";
const HOSTED_ACCOUNT_TABLESTORE_AK_ID_ENV: &str = "OASIS7_HOSTED_ACCOUNT_TABLESTORE_AK_ID";
const HOSTED_ACCOUNT_TABLESTORE_AK_SECRET_ENV: &str = "OASIS7_HOSTED_ACCOUNT_TABLESTORE_AK_SECRET";
const HOSTED_ACCOUNT_TABLESTORE_STS_TOKEN_ENV: &str = "OASIS7_HOSTED_ACCOUNT_TABLESTORE_STS_TOKEN";
const HOSTED_ACCOUNT_TABLESTORE_TABLE_ENV: &str = "OASIS7_HOSTED_ACCOUNT_TABLESTORE_TABLE";
const HOSTED_ACCOUNT_TABLESTORE_AUTO_CREATE_ENV: &str =
    "OASIS7_HOSTED_ACCOUNT_TABLESTORE_AUTO_CREATE";
const ALIYUN_OTS_ENDPOINT_ENV: &str = "ALIYUN_OTS_ENDPOINT";
const ALIYUN_OTS_AK_ID_ENV: &str = "ALIYUN_OTS_AK_ID";
const ALIYUN_OTS_AK_SECRET_ENV: &str = "ALIYUN_OTS_AK_SEC";
const ALIYUN_OTS_STS_TOKEN_ENV: &str = "ALIYUN_OTS_STS_TOKEN";
const HOSTED_ACCOUNT_TABLESTORE_DEFAULT_TABLE: &str = "oasis7_hosted_account_identity";
const HOSTED_ACCOUNT_TABLESTORE_FACTOR_BUCKET: &str = "factor";
const HOSTED_ACCOUNT_TABLESTORE_META_BUCKET: &str = "meta";
const HOSTED_ACCOUNT_TABLESTORE_SEQUENCE_KEY: &str = "sequence";
const HOSTED_ACCOUNT_TABLESTORE_PK_BUCKET: &str = "bucket";
const HOSTED_ACCOUNT_TABLESTORE_PK_KEY: &str = "key";
const HOSTED_ACCOUNT_TABLESTORE_COL_ACCOUNT_ID: &str = "hosted_account_id";
const HOSTED_ACCOUNT_TABLESTORE_COL_PLAYER_ID: &str = "player_id";
const HOSTED_ACCOUNT_TABLESTORE_COL_LOGIN_CHANNEL: &str = "login_channel";
const HOSTED_ACCOUNT_TABLESTORE_COL_NORMALIZED_LOGIN_HINT: &str = "normalized_login_hint";
const HOSTED_ACCOUNT_TABLESTORE_COL_MASKED_LOGIN_HINT: &str = "masked_login_hint";
const HOSTED_ACCOUNT_TABLESTORE_COL_STATUS: &str = "status";
const HOSTED_ACCOUNT_TABLESTORE_COL_CREATED_AT_UNIX_MS: &str = "created_at_unix_ms";
const HOSTED_ACCOUNT_TABLESTORE_COL_LAST_VERIFIED_AT_UNIX_MS: &str = "last_verified_at_unix_ms";
const HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_ACCOUNT_SEQUENCE: &str = "next_account_sequence";
const HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_PLAYER_SEQUENCE: &str = "next_player_sequence";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HostedAccountStore {
    next_account_sequence: u64,
    next_player_sequence: u64,
    accounts_by_id: std::collections::BTreeMap<String, HostedAccountRecord>,
    account_id_by_factor: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct HostedAccountRecord {
    pub(super) hosted_account_id: String,
    pub(super) player_id: String,
    pub(super) login_channel: String,
    pub(super) normalized_login_hint: String,
    pub(super) masked_login_hint: String,
    pub(super) status: String,
    pub(super) created_at_unix_ms: u64,
    pub(super) last_verified_at_unix_ms: u64,
}

pub(super) enum HostedAccountStoreBackend {
    Disabled,
    File(FileHostedAccountStoreBackend),
    Tablestore(HostedAccountTablestoreBackend),
}

#[derive(Debug)]
pub(super) struct FileHostedAccountStoreBackend {
    store_path: PathBuf,
    store: HostedAccountStore,
}

pub(super) struct HostedAccountTablestoreBackend {
    table_name: String,
    auto_create: bool,
    client: OtsClient,
    runtime: Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostedAccountStoreBackendMode {
    File,
    Tablestore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedAccountTablestoreConfig {
    endpoint: String,
    access_key_id: String,
    access_key_secret: String,
    sts_token: Option<String>,
    table_name: String,
    auto_create: bool,
}

impl std::fmt::Debug for HostedAccountStoreBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => f.write_str("HostedAccountStoreBackend::Disabled"),
            Self::File(inner) => f
                .debug_tuple("HostedAccountStoreBackend::File")
                .field(inner)
                .finish(),
            Self::Tablestore(inner) => f
                .debug_tuple("HostedAccountStoreBackend::Tablestore")
                .field(inner)
                .finish(),
        }
    }
}

impl std::fmt::Debug for HostedAccountTablestoreBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostedAccountTablestoreBackend")
            .field("table_name", &self.table_name)
            .field("auto_create", &self.auto_create)
            .field("client", &self.client)
            .finish()
    }
}

impl HostedAccountStoreBackend {
    pub(super) fn from_env() -> Result<Self, String> {
        let mode = HostedAccountStoreBackendMode::from_lookup(|key| std::env::var(key).ok())?;
        match mode {
            HostedAccountStoreBackendMode::File => {
                FileHostedAccountStoreBackend::from_env().map(Self::File)
            }
            HostedAccountStoreBackendMode::Tablestore => {
                HostedAccountTablestoreBackend::from_env().map(Self::Tablestore)
            }
        }
    }

    #[cfg(test)]
    pub(super) fn with_file_store_path(store_path: PathBuf) -> Result<Self, String> {
        FileHostedAccountStoreBackend::with_store_path(store_path).map(Self::File)
    }

    pub(super) fn disabled() -> Self {
        Self::Disabled
    }

    pub(super) fn record_verified_login(
        &mut self,
        factor_key: &str,
        login_channel: &str,
        normalized_login_hint: &str,
        masked_login_hint: &str,
        verified_at_unix_ms: u64,
    ) -> Result<HostedAccountRecord, String> {
        match self {
            Self::Disabled => Err(
                "hosted account store backend is disabled outside hosted_public_join".to_string(),
            ),
            Self::File(inner) => inner.record_verified_login(
                factor_key,
                login_channel,
                normalized_login_hint,
                masked_login_hint,
                verified_at_unix_ms,
            ),
            Self::Tablestore(inner) => inner.record_verified_login(
                factor_key,
                login_channel,
                normalized_login_hint,
                masked_login_hint,
                verified_at_unix_ms,
            ),
        }
    }

    #[cfg(test)]
    pub(super) fn debug_account_count(&self) -> usize {
        match self {
            Self::Disabled => 0,
            Self::File(inner) => inner.store.accounts_by_id.len(),
            Self::Tablestore(_) => 0,
        }
    }
}

impl FileHostedAccountStoreBackend {
    fn from_env() -> Result<Self, String> {
        let store_path = resolve_store_path();
        let store = load_store(store_path.as_path())?;
        Ok(Self { store_path, store })
    }

    #[cfg(test)]
    fn with_store_path(store_path: PathBuf) -> Result<Self, String> {
        let store = load_store(store_path.as_path())?;
        Ok(Self { store_path, store })
    }

    fn record_verified_login(
        &mut self,
        factor_key: &str,
        login_channel: &str,
        normalized_login_hint: &str,
        masked_login_hint: &str,
        verified_at_unix_ms: u64,
    ) -> Result<HostedAccountRecord, String> {
        let account_id = self
            .store
            .account_id_by_factor
            .get(factor_key)
            .cloned()
            .unwrap_or_else(|| {
                self.store.next_account_sequence =
                    self.store.next_account_sequence.saturating_add(1);
                build_hosted_account_id(self.store.next_account_sequence)
            });
        let record = if let Some(record) = self.store.accounts_by_id.get_mut(account_id.as_str()) {
            record.last_verified_at_unix_ms = verified_at_unix_ms;
            record.status = "active".to_string();
            record.clone()
        } else {
            self.store.next_player_sequence = self.store.next_player_sequence.saturating_add(1);
            let player_id = build_hosted_player_id(self.store.next_player_sequence);
            let record = HostedAccountRecord {
                hosted_account_id: account_id.clone(),
                player_id,
                login_channel: login_channel.to_string(),
                normalized_login_hint: normalized_login_hint.to_string(),
                masked_login_hint: masked_login_hint.to_string(),
                status: "active".to_string(),
                created_at_unix_ms: verified_at_unix_ms,
                last_verified_at_unix_ms: verified_at_unix_ms,
            };
            self.store
                .accounts_by_id
                .insert(account_id.clone(), record.clone());
            self.store
                .account_id_by_factor
                .insert(factor_key.to_string(), account_id);
            record
        };
        save_store(self.store_path.as_path(), &self.store)?;
        Ok(record)
    }
}

impl HostedAccountTablestoreBackend {
    fn from_env() -> Result<Self, String> {
        let config = HostedAccountTablestoreConfig::from_lookup(|key| std::env::var(key).ok())?;
        Self::from_config(config)
    }

    fn from_config(config: HostedAccountTablestoreConfig) -> Result<Self, String> {
        let mut builder = OtsClient::builder(
            config.access_key_id.as_str(),
            config.access_key_secret.as_str(),
        )
        .endpoint(config.endpoint.as_str());
        if let Some(token) = config.sts_token.as_deref() {
            builder = builder.sts_token(token);
        }
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| format!("failed to build hosted account tablestore runtime: {err}"))?;
        let mut backend = Self {
            table_name: config.table_name,
            auto_create: config.auto_create,
            client: builder.build(),
            runtime,
        };
        backend.ensure_table_ready()?;
        Ok(backend)
    }

    fn ensure_table_ready(&mut self) -> Result<(), String> {
        if self.table_exists()? {
            return Ok(());
        }
        if !self.auto_create {
            return Err(format!(
                "hosted account tablestore table `{}` does not exist and auto create is disabled",
                self.table_name
            ));
        }
        let request = CreateTableRequest::new(self.table_name.as_str())
            .primary_key_string(HOSTED_ACCOUNT_TABLESTORE_PK_BUCKET)
            .primary_key_string(HOSTED_ACCOUNT_TABLESTORE_PK_KEY)
            .ttl_seconds(-1)
            .max_versions(1)
            .allow_update(true);
        let client = self.client.clone();
        match self.run_ots("create hosted account tablestore table", async move {
            client.create_table(request).send().await
        }) {
            Ok(_) => Ok(()),
            Err(err) if err.contains("AlreadyExist") || err.contains("already exist") => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn table_exists(&mut self) -> Result<bool, String> {
        let client = self.client.clone();
        let table_name = self.table_name.clone();
        let tables = self.run_ots("list hosted account tablestore tables", async move {
            client.list_table().send().await
        })?;
        Ok(tables.iter().any(|name| name == table_name.as_str()))
    }

    fn record_verified_login(
        &mut self,
        factor_key: &str,
        login_channel: &str,
        normalized_login_hint: &str,
        masked_login_hint: &str,
        verified_at_unix_ms: u64,
    ) -> Result<HostedAccountRecord, String> {
        if let Some(existing) = self.get_factor_record(factor_key)? {
            return self.touch_existing_factor_record(
                factor_key,
                existing,
                masked_login_hint,
                verified_at_unix_ms,
            );
        }

        let (account_sequence, player_sequence) = self.allocate_sequences()?;
        let record = HostedAccountRecord {
            hosted_account_id: build_hosted_account_id(account_sequence),
            player_id: build_hosted_player_id(player_sequence),
            login_channel: login_channel.to_string(),
            normalized_login_hint: normalized_login_hint.to_string(),
            masked_login_hint: masked_login_hint.to_string(),
            status: "active".to_string(),
            created_at_unix_ms: verified_at_unix_ms,
            last_verified_at_unix_ms: verified_at_unix_ms,
        };
        if self.put_factor_record(factor_key, &record)? {
            return Ok(record);
        }
        let existing = self.get_factor_record(factor_key)?.ok_or_else(|| {
            "hosted account tablestore factor row conflicted but could not be reloaded".to_string()
        })?;
        self.touch_existing_factor_record(
            factor_key,
            existing,
            masked_login_hint,
            verified_at_unix_ms,
        )
    }

    fn get_factor_record(
        &mut self,
        factor_key: &str,
    ) -> Result<Option<HostedAccountRecord>, String> {
        let request = GetRowRequest::new(self.table_name.as_str())
            .primary_key_column_string(
                HOSTED_ACCOUNT_TABLESTORE_PK_BUCKET,
                HOSTED_ACCOUNT_TABLESTORE_FACTOR_BUCKET,
            )
            .primary_key_column_string(HOSTED_ACCOUNT_TABLESTORE_PK_KEY, factor_key)
            .max_versions(1);
        let client = self.client.clone();
        let response = self.run_ots("get hosted account factor row", async move {
            client.get_row(request).send().await
        })?;
        response.row.map(hosted_account_record_from_row).transpose()
    }

    fn touch_existing_factor_record(
        &mut self,
        factor_key: &str,
        mut record: HostedAccountRecord,
        masked_login_hint: &str,
        verified_at_unix_ms: u64,
    ) -> Result<HostedAccountRecord, String> {
        record.masked_login_hint = masked_login_hint.to_string();
        record.status = "active".to_string();
        record.last_verified_at_unix_ms = verified_at_unix_ms;
        let row = factor_row(factor_key, &record);
        let request = UpdateRowRequest::new(self.table_name.as_str())
            .row(row)
            .row_condition(RowExistenceExpectation::ExpectExist);
        let client = self.client.clone();
        self.run_ots("update hosted account factor row", async move {
            client.update_row(request).send().await
        })?;
        Ok(record)
    }

    fn put_factor_record(
        &mut self,
        factor_key: &str,
        record: &HostedAccountRecord,
    ) -> Result<bool, String> {
        let row = factor_row(factor_key, record);
        let request = PutRowRequest::new(self.table_name.as_str())
            .row(row)
            .row_condition(RowExistenceExpectation::ExpectNotExist);
        let client = self.client.clone();
        match self.run_ots("create hosted account factor row", async move {
            client.put_row(request).send().await
        }) {
            Ok(_) => Ok(true),
            Err(err) if err.contains("OTSConditionCheckFail") => Ok(false),
            Err(err) => Err(err),
        }
    }

    fn allocate_sequences(&mut self) -> Result<(u64, u64), String> {
        let row = Row::new()
            .primary_key_column_string(
                HOSTED_ACCOUNT_TABLESTORE_PK_BUCKET,
                HOSTED_ACCOUNT_TABLESTORE_META_BUCKET,
            )
            .primary_key_column_string(
                HOSTED_ACCOUNT_TABLESTORE_PK_KEY,
                HOSTED_ACCOUNT_TABLESTORE_SEQUENCE_KEY,
            )
            .column_to_increse(HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_ACCOUNT_SEQUENCE, 1)
            .column_to_increse(HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_PLAYER_SEQUENCE, 1);
        let request = UpdateRowRequest::new(self.table_name.as_str())
            .row(row)
            .return_type(ReturnType::RtAfterModify)
            .return_column(HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_ACCOUNT_SEQUENCE)
            .return_column(HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_PLAYER_SEQUENCE);
        let client = self.client.clone();
        let response = self.run_ots("increment hosted account sequences", async move {
            client.update_row(request).send().await
        })?;
        let row = response.row.ok_or_else(|| {
            "hosted account tablestore sequence update returned no modified row".to_string()
        })?;
        let account_sequence =
            required_i64_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_ACCOUNT_SEQUENCE)?;
        let player_sequence =
            required_i64_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_NEXT_PLAYER_SEQUENCE)?;
        let account_sequence = u64::try_from(account_sequence).map_err(|err| {
            format!("hosted account tablestore next account sequence is invalid: {err}")
        })?;
        let player_sequence = u64::try_from(player_sequence).map_err(|err| {
            format!("hosted account tablestore next player sequence is invalid: {err}")
        })?;
        Ok((account_sequence, player_sequence))
    }

    fn run_ots<T, Fut>(&mut self, context: &str, future: Fut) -> Result<T, String>
    where
        Fut: Future<Output = Result<T, OtsError>>,
    {
        self.runtime
            .block_on(future)
            .map_err(|err| format!("{context} failed: {err}"))
    }
}

impl HostedAccountStoreBackendMode {
    fn from_lookup<F>(mut lookup: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let raw = optional_trimmed_lookup(&mut lookup, HOSTED_ACCOUNT_STORE_BACKEND_ENV)
            .unwrap_or_else(|| HOSTED_ACCOUNT_STORE_BACKEND_AUTO.to_string());
        match raw.as_str() {
            HOSTED_ACCOUNT_STORE_BACKEND_AUTO => {
                if has_tablestore_lookup(&mut lookup) {
                    Ok(Self::Tablestore)
                } else {
                    Ok(Self::File)
                }
            }
            HOSTED_ACCOUNT_STORE_BACKEND_FILE => Ok(Self::File),
            HOSTED_ACCOUNT_STORE_BACKEND_TABLESTORE => Ok(Self::Tablestore),
            _ => Err(format!(
                "unsupported hosted account store backend `{raw}`; expected auto, file, or tablestore"
            )),
        }
    }
}

impl HostedAccountTablestoreConfig {
    fn from_lookup<F>(mut lookup: F) -> Result<Self, String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let endpoint = required_trimmed_lookup_any(
            &mut lookup,
            &[
                HOSTED_ACCOUNT_TABLESTORE_ENDPOINT_ENV,
                ALIYUN_OTS_ENDPOINT_ENV,
            ],
            "hosted account tablestore endpoint",
        )?;
        let access_key_id = required_trimmed_lookup_any(
            &mut lookup,
            &[HOSTED_ACCOUNT_TABLESTORE_AK_ID_ENV, ALIYUN_OTS_AK_ID_ENV],
            "hosted account tablestore access key id",
        )?;
        let access_key_secret = required_trimmed_lookup_any(
            &mut lookup,
            &[
                HOSTED_ACCOUNT_TABLESTORE_AK_SECRET_ENV,
                ALIYUN_OTS_AK_SECRET_ENV,
            ],
            "hosted account tablestore access key secret",
        )?;
        let sts_token = optional_trimmed_lookup_any(
            &mut lookup,
            &[
                HOSTED_ACCOUNT_TABLESTORE_STS_TOKEN_ENV,
                ALIYUN_OTS_STS_TOKEN_ENV,
            ],
        );
        let table_name = optional_trimmed_lookup(&mut lookup, HOSTED_ACCOUNT_TABLESTORE_TABLE_ENV)
            .unwrap_or_else(|| HOSTED_ACCOUNT_TABLESTORE_DEFAULT_TABLE.to_string());
        let auto_create =
            parse_bool_lookup(&mut lookup, HOSTED_ACCOUNT_TABLESTORE_AUTO_CREATE_ENV, true)?;
        Ok(Self {
            endpoint,
            access_key_id,
            access_key_secret,
            sts_token,
            table_name,
            auto_create,
        })
    }
}

fn resolve_store_path() -> PathBuf {
    if let Ok(raw) = std::env::var(HOSTED_ACCOUNT_STORE_PATH_ENV) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".oasis7-hosted-account-store.json")
}

fn load_store(path: &Path) -> Result<HostedAccountStore, String> {
    if !path.exists() {
        return Ok(HostedAccountStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read hosted account store `{}`: {err}",
            path.display()
        )
    })?;
    serde_json::from_str(raw.as_str()).map_err(|err| {
        format!(
            "failed to parse hosted account store `{}`: {err}",
            path.display()
        )
    })
}

fn save_store(path: &Path, store: &HostedAccountStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create hosted account store directory `{}`: {err}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("failed to serialize hosted account store: {err}"))?;
    #[cfg(unix)]
    {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|err| {
                format!(
                    "failed to open hosted account store `{}`: {err}",
                    path.display()
                )
            })?;
        file.write_all(raw.as_bytes()).map_err(|err| {
            format!(
                "failed to write hosted account store `{}`: {err}",
                path.display()
            )
        })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|err| {
                format!(
                    "failed to secure hosted account store permissions `{}`: {err}",
                    path.display()
                )
            })?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        fs::write(path, raw).map_err(|err| {
            format!(
                "failed to write hosted account store `{}`: {err}",
                path.display()
            )
        })
    }
}

fn build_hosted_account_id(sequence: u64) -> String {
    format!("oasis-account-{sequence:08x}")
}

fn build_hosted_player_id(sequence: u64) -> String {
    format!("hosted-player-account-{sequence:08x}")
}

fn factor_row(factor_key: &str, record: &HostedAccountRecord) -> Row {
    Row::new()
        .primary_key_column_string(
            HOSTED_ACCOUNT_TABLESTORE_PK_BUCKET,
            HOSTED_ACCOUNT_TABLESTORE_FACTOR_BUCKET,
        )
        .primary_key_column_string(HOSTED_ACCOUNT_TABLESTORE_PK_KEY, factor_key)
        .column_string(
            HOSTED_ACCOUNT_TABLESTORE_COL_ACCOUNT_ID,
            record.hosted_account_id.as_str(),
        )
        .column_string(
            HOSTED_ACCOUNT_TABLESTORE_COL_PLAYER_ID,
            record.player_id.as_str(),
        )
        .column_string(
            HOSTED_ACCOUNT_TABLESTORE_COL_LOGIN_CHANNEL,
            record.login_channel.as_str(),
        )
        .column_string(
            HOSTED_ACCOUNT_TABLESTORE_COL_NORMALIZED_LOGIN_HINT,
            record.normalized_login_hint.as_str(),
        )
        .column_string(
            HOSTED_ACCOUNT_TABLESTORE_COL_MASKED_LOGIN_HINT,
            record.masked_login_hint.as_str(),
        )
        .column_string(HOSTED_ACCOUNT_TABLESTORE_COL_STATUS, record.status.as_str())
        .column_integer(
            HOSTED_ACCOUNT_TABLESTORE_COL_CREATED_AT_UNIX_MS,
            unix_ms_to_i64(record.created_at_unix_ms),
        )
        .column_integer(
            HOSTED_ACCOUNT_TABLESTORE_COL_LAST_VERIFIED_AT_UNIX_MS,
            unix_ms_to_i64(record.last_verified_at_unix_ms),
        )
}

fn hosted_account_record_from_row(row: Row) -> Result<HostedAccountRecord, String> {
    Ok(HostedAccountRecord {
        hosted_account_id: required_string_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_ACCOUNT_ID)?,
        player_id: required_string_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_PLAYER_ID)?,
        login_channel: required_string_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_LOGIN_CHANNEL)?,
        normalized_login_hint: required_string_column(
            &row,
            HOSTED_ACCOUNT_TABLESTORE_COL_NORMALIZED_LOGIN_HINT,
        )?,
        masked_login_hint: required_string_column(
            &row,
            HOSTED_ACCOUNT_TABLESTORE_COL_MASKED_LOGIN_HINT,
        )?,
        status: required_string_column(&row, HOSTED_ACCOUNT_TABLESTORE_COL_STATUS)?,
        created_at_unix_ms: required_u64_column(
            &row,
            HOSTED_ACCOUNT_TABLESTORE_COL_CREATED_AT_UNIX_MS,
        )?,
        last_verified_at_unix_ms: required_u64_column(
            &row,
            HOSTED_ACCOUNT_TABLESTORE_COL_LAST_VERIFIED_AT_UNIX_MS,
        )?,
    })
}

fn required_string_column(row: &Row, name: &str) -> Result<String, String> {
    match row.get_column_value(name) {
        Some(ColumnValue::String(value)) => Ok(value.clone()),
        Some(other) => Err(format!(
            "hosted account tablestore column `{name}` has non-string value `{other:?}`"
        )),
        None => Err(format!(
            "hosted account tablestore row is missing required string column `{name}`"
        )),
    }
}

fn required_i64_column(row: &Row, name: &str) -> Result<i64, String> {
    match row.get_column_value(name) {
        Some(ColumnValue::Integer(value)) => Ok(*value),
        Some(other) => Err(format!(
            "hosted account tablestore column `{name}` has non-integer value `{other:?}`"
        )),
        None => Err(format!(
            "hosted account tablestore row is missing required integer column `{name}`"
        )),
    }
}

fn required_u64_column(row: &Row, name: &str) -> Result<u64, String> {
    let value = required_i64_column(row, name)?;
    u64::try_from(value)
        .map_err(|err| format!("hosted account tablestore column `{name}` is negative: {err}"))
}

fn unix_ms_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn has_tablestore_lookup<F>(lookup: &mut F) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    optional_trimmed_lookup_any(
        lookup,
        &[
            HOSTED_ACCOUNT_TABLESTORE_ENDPOINT_ENV,
            ALIYUN_OTS_ENDPOINT_ENV,
            HOSTED_ACCOUNT_TABLESTORE_AK_ID_ENV,
            ALIYUN_OTS_AK_ID_ENV,
        ],
    )
    .is_some()
}

fn optional_trimmed_lookup<F>(lookup: &mut F, key: &str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(key).and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn optional_trimmed_lookup_any<F>(lookup: &mut F, keys: &[&str]) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    for key in keys {
        if let Some(value) = optional_trimmed_lookup(lookup, key) {
            return Some(value);
        }
    }
    None
}

fn required_trimmed_lookup_any<F>(
    lookup: &mut F,
    keys: &[&str],
    label: &str,
) -> Result<String, String>
where
    F: FnMut(&str) -> Option<String>,
{
    optional_trimmed_lookup_any(lookup, keys).ok_or_else(|| {
        format!(
            "{label} env `{}` is required when hosted account tablestore backend is enabled",
            keys.join("` or `")
        )
    })
}

fn parse_bool_lookup<F>(lookup: &mut F, key: &str, default: bool) -> Result<bool, String>
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(raw) = optional_trimmed_lookup(lookup, key) else {
        return Ok(default);
    };
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "hosted account boolean env `{key}` has invalid value `{raw}`"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_account_store_backend_mode_defaults_to_file() {
        let mode = HostedAccountStoreBackendMode::from_lookup(|_| None).expect("mode");
        assert_eq!(mode, HostedAccountStoreBackendMode::File);
    }

    #[test]
    fn hosted_account_store_backend_mode_auto_detects_tablestore() {
        let mode = HostedAccountStoreBackendMode::from_lookup(|key| match key {
            HOSTED_ACCOUNT_TABLESTORE_ENDPOINT_ENV => {
                Some("https://instance.cn-hangzhou.ots.aliyuncs.com".to_string())
            }
            _ => None,
        })
        .expect("mode");
        assert_eq!(mode, HostedAccountStoreBackendMode::Tablestore);
    }

    #[test]
    fn hosted_account_tablestore_config_uses_generic_env_fallbacks() {
        let config = HostedAccountTablestoreConfig::from_lookup(|key| match key {
            ALIYUN_OTS_ENDPOINT_ENV => {
                Some("https://game-login.cn-hangzhou.ots.aliyuncs.com".to_string())
            }
            ALIYUN_OTS_AK_ID_ENV => Some("akid".to_string()),
            ALIYUN_OTS_AK_SECRET_ENV => Some("aksecret".to_string()),
            _ => None,
        })
        .expect("config");
        assert_eq!(
            config.endpoint,
            "https://game-login.cn-hangzhou.ots.aliyuncs.com"
        );
        assert_eq!(config.access_key_id, "akid");
        assert_eq!(config.access_key_secret, "aksecret");
        assert_eq!(
            config.table_name,
            HOSTED_ACCOUNT_TABLESTORE_DEFAULT_TABLE.to_string()
        );
        assert!(config.auto_create);
    }

    #[test]
    fn hosted_account_tablestore_config_respects_dedicated_overrides() {
        let config = HostedAccountTablestoreConfig::from_lookup(|key| match key {
            HOSTED_ACCOUNT_TABLESTORE_ENDPOINT_ENV => {
                Some("https://custom.cn-beijing.ots.aliyuncs.com".to_string())
            }
            HOSTED_ACCOUNT_TABLESTORE_AK_ID_ENV => Some("custom-ak".to_string()),
            HOSTED_ACCOUNT_TABLESTORE_AK_SECRET_ENV => Some("custom-secret".to_string()),
            HOSTED_ACCOUNT_TABLESTORE_TABLE_ENV => Some("login_accounts".to_string()),
            HOSTED_ACCOUNT_TABLESTORE_AUTO_CREATE_ENV => Some("false".to_string()),
            HOSTED_ACCOUNT_TABLESTORE_STS_TOKEN_ENV => Some("sts".to_string()),
            _ => None,
        })
        .expect("config");
        assert_eq!(
            config.endpoint,
            "https://custom.cn-beijing.ots.aliyuncs.com"
        );
        assert_eq!(config.access_key_id, "custom-ak");
        assert_eq!(config.access_key_secret, "custom-secret");
        assert_eq!(config.table_name, "login_accounts");
        assert_eq!(config.sts_token.as_deref(), Some("sts"));
        assert!(!config.auto_create);
    }
}
