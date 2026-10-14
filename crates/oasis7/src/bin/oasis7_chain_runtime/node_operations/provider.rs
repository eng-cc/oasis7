//! Fixed-command SSH/systemd adapter and its remote action agent.
//!
//! The SSH command line is constant; all request data travels as bounded JSON
//! on stdin. A remote action intent is persisted before systemd is touched.
//! A pending intent is query-only after restart.
use super::super::node_lifecycle::{LocalInstance, actions, local_io as io};
use super::model::{LinuxSshSystemdTargetV1, ManagedTargetV1, Record};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
#[cfg(target_os = "linux")]
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_AGENT_MESSAGE: usize = 64 * 1024;
#[cfg(target_os = "linux")]
const MAX_AGENT_REPLY: usize = 1024 * 1024;
#[cfg(target_os = "linux")]
const AGENT_COMMAND: &str = "/usr/bin/oasis7_chain_runtime op agent";
const NODE_EXECUTABLE: &str = "/usr/bin/oasis7_chain_runtime";
const SYSTEMCTL: &str = "/usr/bin/systemctl";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RemoteStepV1 {
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RemoteOutcomeV1 {
    Pending,
    Running,
    Stopped,
    Unknown,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RemoteActionReceiptV1 {
    pub api_version: String,
    pub operation_id: String,
    pub action_id: String,
    pub request_digest: String,
    pub operation_request_digest: String,
    pub node_id: String,
    pub world_id: String,
    pub chain_id: String,
    pub step: RemoteStepV1,
    pub outcome: RemoteOutcomeV1,
    pub expected_generation: u64,
    pub expected_instance: Option<LocalInstance>,
    pub instance: Option<LocalInstance>,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentTargetV1 {
    node_id: String,
    service_user: String,
    unit: String,
    config: PathBuf,
    data_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentModeV1 {
    Query,
    Apply,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentRequestV1 {
    api_version: String,
    mode: AgentModeV1,
    operation_id: String,
    action_id: String,
    request_digest: String,
    operation_request_digest: String,
    operation_request_json: String,
    world_id: String,
    chain_id: String,
    target: AgentTargetV1,
    step: RemoteStepV1,
    expected_generation: u64,
    expected_config_digest: String,
    expected_instance: Option<LocalInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentReplyV1 {
    api_version: String,
    receipt: Option<RemoteActionReceiptV1>,
}

pub(super) trait AgentTransport {
    fn exchange(
        &mut self,
        target: &LinuxSshSystemdTargetV1,
        message: &[u8],
    ) -> Result<Vec<u8>, String>;
}

#[path = "provider_agent.rs"]
mod agent;
#[cfg(test)]
use agent::{Agent, SystemdRunner, receipt, validate_unit_contents};
pub(super) use agent::{run_agent_stdin, validate_receipt};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct FakeAgentRunner {
        active: bool,
        effect_count: usize,
    }

    impl SystemdRunner for FakeAgentRunner {
        fn validate_unit(&mut self, _target: &AgentTargetV1) -> Result<(), String> {
            Ok(())
        }

        fn active(&mut self, _unit: &str) -> Result<bool, String> {
            Ok(self.active)
        }

        fn effect(&mut self, request: &AgentRequestV1) -> Result<(), String> {
            self.effect_count += 1;
            match request.step {
                RemoteStepV1::Start => {
                    actions::start(
                        &request.target.config,
                        &request.action_id,
                        request.expected_generation,
                        &request.expected_config_digest,
                    )?;
                    self.active = true;
                }
                RemoteStepV1::Stop => {
                    let expected = request
                        .expected_instance
                        .as_ref()
                        .ok_or("stop target missing")?;
                    actions::stop(
                        &request.target.data_dir,
                        &request.action_id,
                        expected,
                        2_000,
                    )?;
                    self.active = false;
                }
            }
            Ok(())
        }
    }

    struct LocalAgentTransport {
        runner: FakeAgentRunner,
        lose_apply_reply: bool,
    }

    impl AgentTransport for LocalAgentTransport {
        fn exchange(
            &mut self,
            _target: &LinuxSshSystemdTargetV1,
            message: &[u8],
        ) -> Result<Vec<u8>, String> {
            let request: AgentRequestV1 =
                serde_json::from_slice(message).map_err(|error| error.to_string())?;
            let response = AgentReplyV1 {
                api_version: "oasis7.remote-operation-agent-reply/v1".into(),
                receipt: Agent::with_runner(request.clone(), &mut self.runner)?,
            };
            let bytes = serde_json::to_vec(&response).map_err(|error| error.to_string())?;
            if request.mode == AgentModeV1::Apply && self.lose_apply_reply {
                self.lose_apply_reply = false;
                return Err("simulated dropped agent response".into());
            }
            Ok(bytes)
        }
    }

    struct FakeSshSystemd {
        current_generation: u64,
        receipt: Option<RemoteActionReceiptV1>,
        apply_count: usize,
        lose_first_apply_reply: bool,
    }

    impl AgentTransport for FakeSshSystemd {
        fn exchange(
            &mut self,
            _target: &LinuxSshSystemdTargetV1,
            message: &[u8],
        ) -> Result<Vec<u8>, String> {
            let request: AgentRequestV1 =
                serde_json::from_slice(message).map_err(|error| error.to_string())?;
            let receipt = match request.mode {
                AgentModeV1::Query => self.receipt.clone(),
                AgentModeV1::Apply => {
                    self.apply_count += 1;
                    if request.expected_generation != self.current_generation {
                        Some(receipt(
                            &request,
                            RemoteOutcomeV1::Rejected,
                            None,
                            Some("STALE_GENERATION".into()),
                        ))
                    } else {
                        let instance = match request.step {
                            RemoteStepV1::Start => {
                                self.current_generation += 1;
                                Some(LocalInstance {
                                    api_version: "oasis7.local-node-instance/v1".into(),
                                    action_id: request.action_id.clone(),
                                    generation: self.current_generation,
                                    incarnation: "0123456789abcdef0123456789abcdef".into(),
                                    config_digest: request.expected_config_digest.clone(),
                                    supervisor_pid: 44,
                                    runtime_pid: Some(45),
                                    started_unix_ms: 7,
                                    state: "running".into(),
                                    exit_code: None,
                                })
                            }
                            RemoteStepV1::Stop => {
                                request.expected_instance.clone().map(|mut instance| {
                                    instance.state = "stopped".into();
                                    instance.runtime_pid = None;
                                    instance
                                })
                            }
                        };
                        let outcome = match request.step {
                            RemoteStepV1::Start => RemoteOutcomeV1::Running,
                            RemoteStepV1::Stop => RemoteOutcomeV1::Stopped,
                        };
                        self.receipt = Some(receipt(&request, outcome, instance, None));
                        if self.lose_first_apply_reply {
                            self.lose_first_apply_reply = false;
                            return Err("simulated lost SSH reply".into());
                        }
                        self.receipt.clone()
                    }
                }
            };
            serde_json::to_vec(&AgentReplyV1 {
                api_version: "oasis7.remote-operation-agent-reply/v1".into(),
                receipt,
            })
            .map_err(|error| error.to_string())
        }
    }

    fn target() -> LinuxSshSystemdTargetV1 {
        LinuxSshSystemdTargetV1 {
            host: "node-a.example.net".into(),
            username: "oasis7".into(),
            port: 22,
            identity_file: PathBuf::from("/unused/test-key"),
            known_hosts_file: PathBuf::from("/unused/test-known-hosts"),
            unit: "oasis7-node-a.service".into(),
            node_id: "node-a".into(),
            remote_config: PathBuf::from("/var/lib/oasis7/node-a/node.json"),
            remote_data_dir: PathBuf::from("/var/lib/oasis7/node-a"),
            timeout_ms: 10_000,
        }
    }

    fn request(step: RemoteStepV1) -> AgentRequestV1 {
        use super::super::model::{Kind, ManagedTargetV1, Request};
        let target = target();
        let expected_instance = (step == RemoteStepV1::Stop).then(|| LocalInstance {
            api_version: "oasis7.local-node-instance/v1".into(),
            action_id: "native-start-3".into(),
            generation: 3,
            incarnation: "0123456789abcdef0123456789abcdef".into(),
            config_digest: "b".repeat(64),
            supervisor_pid: 44,
            runtime_pid: Some(45),
            started_unix_ms: 7,
            state: "running".into(),
            exit_code: None,
        });
        let original = Request {
            api_version: "oasis7.local-operation/v1".into(),
            operation_id: "op-loss".into(),
            world_id: "world-a".into(),
            chain_id: "chain-a".into(),
            kind: if step == RemoteStepV1::Start {
                Kind::Start
            } else {
                Kind::Stop
            },
            config: target.remote_config.clone(),
            expected_generation: 3,
            expected_config_digest: "b".repeat(64),
            expected_instance: expected_instance.clone(),
            managed_target: Some(ManagedTargetV1::LinuxSshSystemd(target.clone())),
            migration: None,
            authorization: None,
            package_upgrade: None,
        };
        let operation_request_json = serde_json::to_string(&original).unwrap();
        let operation_request_digest = io::digest(&original).unwrap();
        let mut request = AgentRequestV1 {
            api_version: "oasis7.remote-operation-agent/v1".into(),
            mode: AgentModeV1::Query,
            operation_id: "op-loss".into(),
            action_id: format!(
                "op:op-loss:{}",
                if step == RemoteStepV1::Start {
                    "start"
                } else {
                    "stop"
                }
            ),
            request_digest: String::new(),
            operation_request_digest,
            operation_request_json,
            world_id: "world-a".into(),
            chain_id: "chain-a".into(),
            target: AgentTargetV1 {
                node_id: target.node_id.clone(),
                service_user: target.username.clone(),
                unit: target.unit.clone(),
                config: target.remote_config.clone(),
                data_dir: target.remote_data_dir.clone(),
            },
            step,
            expected_generation: 3,
            expected_config_digest: "b".repeat(64),
            expected_instance,
        };
        request.request_digest = action_digest(
            &request.operation_request_digest,
            &request.operation_id,
            &request.action_id,
            &request.world_id,
            &request.chain_id,
            &request.target,
            request.step,
            request.expected_generation,
            &request.expected_config_digest,
            &request.expected_instance,
        )
        .unwrap();
        request
    }

    fn local_stop_request(
        config: &std::path::Path,
        instance: &LocalInstance,
    ) -> (LinuxSshSystemdTargetV1, AgentRequestV1) {
        let binding = actions::binding(config).unwrap();
        let node: serde_json::Value = io::read(config).unwrap();
        use super::super::model::{Kind, ManagedTargetV1, Request};
        let target = LinuxSshSystemdTargetV1 {
            host: "node-a.example.net".into(),
            username: "oasis7".into(),
            port: 22,
            identity_file: PathBuf::from("/unused/test-key"),
            known_hosts_file: PathBuf::from("/unused/test-known-hosts"),
            unit: "oasis7-node-test.service".into(),
            node_id: node["identity"]["node_id"].as_str().unwrap().into(),
            remote_config: config.to_path_buf(),
            remote_data_dir: binding.data_dir.clone(),
            timeout_ms: 10_000,
        };
        let original = Request {
            api_version: "oasis7.local-operation/v1".into(),
            operation_id: "op-agent-lost-reply".into(),
            world_id: binding.world_id.clone(),
            chain_id: binding.chain_id.clone(),
            kind: Kind::Stop,
            config: config.to_path_buf(),
            expected_generation: instance.generation,
            expected_config_digest: binding.config_digest.clone(),
            expected_instance: Some(instance.clone()),
            managed_target: Some(ManagedTargetV1::LinuxSshSystemd(target.clone())),
            migration: None,
            authorization: None,
            package_upgrade: None,
        };
        let operation_request_json = serde_json::to_string(&original).unwrap();
        let operation_request_digest = io::digest(&original).unwrap();
        let mut request = AgentRequestV1 {
            api_version: "oasis7.remote-operation-agent/v1".into(),
            mode: AgentModeV1::Query,
            operation_id: "op-agent-lost-reply".into(),
            action_id: "op:op-agent-lost-reply:stop".into(),
            request_digest: String::new(),
            operation_request_digest,
            operation_request_json,
            world_id: binding.world_id,
            chain_id: binding.chain_id,
            target: AgentTargetV1 {
                node_id: node["identity"]["node_id"].as_str().unwrap().into(),
                service_user: target.username.clone(),
                unit: target.unit.clone(),
                config: config.to_path_buf(),
                data_dir: binding.data_dir.clone(),
            },
            step: RemoteStepV1::Stop,
            expected_generation: instance.generation,
            expected_config_digest: binding.config_digest.clone(),
            expected_instance: Some(instance.clone()),
        };
        request.request_digest = action_digest(
            &request.operation_request_digest,
            &request.operation_id,
            &request.action_id,
            &request.world_id,
            &request.chain_id,
            &request.target,
            request.step,
            request.expected_generation,
            &request.expected_config_digest,
            &request.expected_instance,
        )
        .unwrap();
        (target, request)
    }

    fn local_start_request(
        config: &std::path::Path,
        expected_generation: u64,
    ) -> (LinuxSshSystemdTargetV1, AgentRequestV1) {
        use super::super::model::{Kind, ManagedTargetV1, Request};
        let binding = actions::binding(config).unwrap();
        let node: serde_json::Value = io::read(config).unwrap();
        let target = LinuxSshSystemdTargetV1 {
            host: "node-a.example.net".into(),
            username: "oasis7".into(),
            port: 22,
            identity_file: PathBuf::from("/unused/test-key"),
            known_hosts_file: PathBuf::from("/unused/test-known-hosts"),
            unit: "oasis7-node-test.service".into(),
            node_id: node["identity"]["node_id"].as_str().unwrap().into(),
            remote_config: config.to_path_buf(),
            remote_data_dir: binding.data_dir.clone(),
            timeout_ms: 10_000,
        };
        let original = Request {
            api_version: "oasis7.local-operation/v1".into(),
            operation_id: "op-agent-stale-start".into(),
            world_id: binding.world_id.clone(),
            chain_id: binding.chain_id.clone(),
            kind: Kind::Start,
            config: config.to_path_buf(),
            expected_generation,
            expected_config_digest: binding.config_digest.clone(),
            expected_instance: None,
            managed_target: Some(ManagedTargetV1::LinuxSshSystemd(target.clone())),
            migration: None,
            authorization: None,
            package_upgrade: None,
        };
        let operation_request_json = serde_json::to_string(&original).unwrap();
        let operation_request_digest = io::digest(&original).unwrap();
        let mut request = AgentRequestV1 {
            api_version: "oasis7.remote-operation-agent/v1".into(),
            mode: AgentModeV1::Query,
            operation_id: original.operation_id.clone(),
            action_id: format!("op:{}:start", original.operation_id),
            request_digest: String::new(),
            operation_request_digest,
            operation_request_json,
            world_id: original.world_id.clone(),
            chain_id: original.chain_id.clone(),
            target: AgentTargetV1 {
                node_id: target.node_id.clone(),
                service_user: target.username.clone(),
                unit: target.unit.clone(),
                config: target.remote_config.clone(),
                data_dir: target.remote_data_dir.clone(),
            },
            step: RemoteStepV1::Start,
            expected_generation,
            expected_config_digest: binding.config_digest,
            expected_instance: None,
        };
        request.request_digest = action_digest(
            &request.operation_request_digest,
            &request.operation_id,
            &request.action_id,
            &request.world_id,
            &request.chain_id,
            &request.target,
            request.step,
            request.expected_generation,
            &request.expected_config_digest,
            &request.expected_instance,
        )
        .unwrap();
        (target, request)
    }

    #[test]
    fn lost_ssh_reply_is_recovered_by_same_action_query_without_redelivery() {
        let mut request = request(RemoteStepV1::Start);
        let mut fake = FakeSshSystemd {
            current_generation: 3,
            receipt: None,
            apply_count: 0,
            lose_first_apply_reply: true,
        };
        assert!(
            exchange_with(&target(), &request, &mut fake)
                .unwrap()
                .is_none()
        );
        request.mode = AgentModeV1::Apply;
        assert!(exchange_with(&target(), &request, &mut fake).is_err());
        request.mode = AgentModeV1::Query;
        let recovered = exchange_with(&target(), &request, &mut fake)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.outcome, RemoteOutcomeV1::Running);
        assert_eq!(recovered.action_id, request.action_id);
        assert_eq!(recovered.instance.unwrap().generation, 4);
        assert_eq!(fake.apply_count, 1);
    }

    #[test]
    fn stale_generation_receipt_never_changes_remote_instance() {
        let mut request = request(RemoteStepV1::Start);
        request.mode = AgentModeV1::Apply;
        let mut fake = FakeSshSystemd {
            current_generation: 4,
            receipt: None,
            apply_count: 0,
            lose_first_apply_reply: false,
        };
        let rejected = exchange_with(&target(), &request, &mut fake)
            .unwrap()
            .unwrap();
        assert_eq!(rejected.outcome, RemoteOutcomeV1::Rejected);
        assert_eq!(fake.current_generation, 4);
        assert_eq!(fake.apply_count, 1);
    }

    #[test]
    fn durable_agent_recovers_lost_reply_by_query_without_repeating_native_stop() {
        let (_fixture, config, handle, current) =
            super::super::super::node_lifecycle::tests::operation_fixture();
        let (target, mut request) = local_stop_request(&config, &current);
        let mut transport = LocalAgentTransport {
            runner: FakeAgentRunner {
                active: true,
                effect_count: 0,
            },
            lose_apply_reply: true,
        };

        assert!(
            exchange_with(&target, &request, &mut transport)
                .unwrap()
                .is_none()
        );
        request.mode = AgentModeV1::Apply;
        assert!(exchange_with(&target, &request, &mut transport).is_err());
        request.mode = AgentModeV1::Query;
        let recovered = exchange_with(&target, &request, &mut transport)
            .unwrap()
            .unwrap();

        assert_eq!(recovered.outcome, RemoteOutcomeV1::Stopped);
        assert_eq!(recovered.action_id, request.action_id);
        assert_eq!(transport.runner.effect_count, 1);
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn durable_agent_rejects_stale_generation_before_effect_or_intent() {
        let (_fixture, config, handle, current) =
            super::super::super::node_lifecycle::tests::operation_fixture();
        let (_target, mut request) =
            local_start_request(&config, current.generation.saturating_sub(1));
        request.mode = AgentModeV1::Apply;
        let mut runner = FakeAgentRunner {
            active: true,
            effect_count: 0,
        };

        assert!(Agent::with_runner(request.clone(), &mut runner).is_err());
        assert_eq!(runner.effect_count, 0);
        assert!(
            actions::query(&request.target.data_dir, &request.action_id)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            actions::inspect(&request.target.data_dir).unwrap()["instance"]["incarnation"],
            current.incarnation
        );
        actions::stop(
            &request.target.data_dir,
            "cleanup-stale-agent-start",
            &current,
            2000,
        )
        .unwrap();
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn late_stop_action_cannot_stop_a_replacement_incarnation() {
        let (_fixture, config, handle, original) =
            super::super::super::node_lifecycle::tests::operation_fixture();
        let binding = actions::binding(&config).unwrap();
        actions::stop(&binding.data_dir, "setup-replacement", &original, 2000).unwrap();
        handle.join().unwrap().unwrap();
        let (replacement, current) =
            super::super::super::node_lifecycle::tests::operation_replacement_receiver(&config);
        let (_target, mut request) = local_stop_request(&config, &original);
        request.mode = AgentModeV1::Apply;
        let mut runner = FakeAgentRunner {
            active: true,
            effect_count: 0,
        };

        assert!(Agent::with_runner(request.clone(), &mut runner).is_err());
        assert_eq!(runner.effect_count, 0);
        assert_eq!(
            actions::inspect(&binding.data_dir).unwrap()["instance"]["incarnation"],
            current.incarnation
        );
        actions::stop(
            &binding.data_dir,
            "cleanup-replacement-agent-stop",
            &current,
            2000,
        )
        .unwrap();
        replacement.join().unwrap().unwrap();
    }

    #[test]
    fn remote_target_rejects_command_injection_fields_before_transport() {
        let mut target = target();
        target.host = "node-a; touch /tmp/pwned".into();
        assert!(!safe_host(&target.host));
        target.host = "node-a.example.net".into();
        target.unit = "node.service;systemctl stop other.service".into();
        assert!(!safe_unit(&target.unit));
    }

    #[test]
    fn managed_unit_profile_rejects_extra_commands_dependencies_and_wrong_service_user() {
        let target = AgentTargetV1 {
            node_id: "node-a".into(),
            service_user: "oasis7".into(),
            unit: "oasis7-node-a.service".into(),
            config: PathBuf::from("/var/lib/oasis7/node-a/node.json"),
            data_dir: PathBuf::from("/var/lib/oasis7/node-a"),
        };
        let exact = format!(
            "[Unit]\nDescription=Oasis7 observer\n[Service]\nType=oneshot\nRemainAfterExit=yes\nUser=oasis7\nExecStart={NODE_EXECUTABLE} node start --config {}\n[Install]\nWantedBy=multi-user.target\n",
            target.config.display()
        );
        assert!(validate_unit_contents(&target, &exact).is_ok());
        for unsafe_line in [
            "ExecStartPre=/bin/touch /tmp/side-effect\n",
            "After=other.service\n",
            "EnvironmentFile=/etc/oasis7-secrets\n",
            "User=root\n",
        ] {
            let unsafe_contents = exact.replace("ExecStart=", &format!("{unsafe_line}ExecStart="));
            assert!(validate_unit_contents(&target, &unsafe_contents).is_err());
        }
        assert!(
            validate_unit_contents(
                &target,
                &exact.replace("[Service]\n", "[Service]\nExecStart=/bin/true\n")
            )
            .is_err()
        );
    }
}

struct SshTransport;
impl AgentTransport for SshTransport {
    fn exchange(
        &mut self,
        target: &LinuxSshSystemdTargetV1,
        message: &[u8],
    ) -> Result<Vec<u8>, String> {
        validate_target(target)?;
        if message.len() > MAX_AGENT_MESSAGE {
            return Err("remote action request size limit".into());
        }
        #[cfg(not(target_os = "linux"))]
        return Err("Linux SSH/systemd provider is unsupported on this host".into());
        #[cfg(target_os = "linux")]
        {
            let mut child = Command::new("/usr/bin/ssh")
                .env_clear()
                .arg("-T")
                .arg("-p")
                .arg(target.port.to_string())
                .arg("-oBatchMode=yes")
                .arg("-oIdentitiesOnly=yes")
                .arg("-oStrictHostKeyChecking=yes")
                .arg(format!(
                    "-oUserKnownHostsFile={}",
                    target.known_hosts_file.display()
                ))
                .arg("-i")
                .arg(&target.identity_file)
                .arg("--")
                .arg(format!("{}@{}", target.username, target.host))
                // This is a literal command with no request-derived bytes.
                .arg(AGENT_COMMAND)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| format!("start fixed SSH client: {error}"))?;
            let stdout = child.stdout.take().ok_or("SSH stdout unavailable")?;
            let stderr = child.stderr.take().ok_or("SSH stderr unavailable")?;
            let stdout_thread = thread::spawn(move || read_capped(stdout, MAX_AGENT_REPLY));
            let stderr_thread = thread::spawn(move || read_capped(stderr, 16 * 1024));
            let mut stdin = child.stdin.take().ok_or("SSH stdin unavailable")?;
            let request_bytes = message.to_vec();
            let stdin_thread = thread::spawn(move || {
                stdin
                    .write_all(&request_bytes)
                    .map_err(|error| format!("write SSH request: {error}"))
            });
            let timeout = Duration::from_millis(target.timeout_ms);
            let started = Instant::now();
            let status = loop {
                match child.try_wait().map_err(|error| error.to_string())? {
                    Some(status) => break Some(status),
                    None if started.elapsed() < timeout => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                }
            };
            let stdin_result = stdin_thread
                .join()
                .map_err(|_| "SSH stdin writer panicked")?;
            let stdout = stdout_thread
                .join()
                .map_err(|_| "SSH stdout reader panicked")??;
            let stderr = stderr_thread
                .join()
                .map_err(|_| "SSH stderr reader panicked")??;
            let Some(status) = status else {
                return Err("ORIGINAL_OUTCOME_UNKNOWN: SSH deadline elapsed; query the same action identity".into());
            };
            stdin_result?;
            if !status.success() {
                return Err(format!(
                    "ORIGINAL_OUTCOME_UNKNOWN: SSH exited with {status}; {}",
                    String::from_utf8_lossy(&stderr).trim()
                ));
            }
            Ok(stdout)
        }
    }
}

#[cfg(target_os = "linux")]
fn read_capped(reader: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > limit {
        return Err("remote helper response size limit".into());
    }
    Ok(bytes)
}

pub(super) fn validate_target(target: &LinuxSshSystemdTargetV1) -> Result<(), String> {
    validate_target_shape(target)?;
    for path in [&target.identity_file, &target.known_hosts_file] {
        io::reject_symlinks(path)?;
        let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
        let max_bytes = if path == &target.identity_file {
            1024 * 1024
        } else {
            4 * 1024 * 1024
        };
        if !metadata.is_file() || metadata.len() > max_bytes {
            return Err("SSH credential/known-hosts path is not a bounded regular file".into());
        }
    }
    super::super::controlled_live_config::require_private_owned(&target.identity_file, false)?;
    Ok(())
}

fn validate_target_shape(target: &LinuxSshSystemdTargetV1) -> Result<(), String> {
    if target.port == 0
        || !(1..=60_000).contains(&target.timeout_ms)
        || !safe_host(&target.host)
        || !safe_user(&target.username)
        || !safe_unit(&target.unit)
        || !safe_node_id(&target.node_id)
        || !target.remote_config.is_absolute()
        || !target.remote_data_dir.is_absolute()
        || !safe_remote_path(&target.remote_config)
        || !safe_remote_path(&target.remote_data_dir)
        || target.remote_config != target.remote_data_dir.join("node.json")
        || !target.identity_file.is_absolute()
        || !target.known_hosts_file.is_absolute()
    {
        return Err("invalid Linux SSH/systemd target identity or paths".into());
    }
    Ok(())
}

fn safe_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte))
        && !value.starts_with('-')
}
fn safe_user(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
        && !value.starts_with('-')
}
fn safe_unit(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"@_.:-".contains(&byte))
        && !value.starts_with('-')
}
fn safe_node_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}
fn safe_remote_path(path: &Path) -> bool {
    let value = path.to_string_lossy();
    path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
        && value.len() <= 4096
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
}

pub(super) fn request_for(record: &Record, step: RemoteStepV1) -> Result<AgentRequestV1, String> {
    let Some(ManagedTargetV1::LinuxSshSystemd(target)) = &record.request.managed_target else {
        return Err("managed SSH/systemd target missing".into());
    };
    validate_target_shape(target)?;
    let action_id = record.action_id(match step {
        RemoteStepV1::Start => "start",
        RemoteStepV1::Stop => "stop",
    });
    let expected_instance = (step == RemoteStepV1::Stop)
        .then(|| record.request.expected_instance.clone())
        .flatten();
    let operation_request_digest = record.request_digest.clone();
    let operation_request_json =
        serde_json::to_string(&record.request).map_err(|error| error.to_string())?;
    if operation_request_json.len() > MAX_AGENT_MESSAGE / 2 {
        return Err("remote operation request exceeds bounded agent envelope".into());
    }
    let agent_target = AgentTargetV1 {
        node_id: target.node_id.clone(),
        service_user: target.username.clone(),
        unit: target.unit.clone(),
        config: target.remote_config.clone(),
        data_dir: target.remote_data_dir.clone(),
    };
    let request_digest = action_digest(
        &operation_request_digest,
        &record.request.operation_id,
        &action_id,
        &record.request.world_id,
        &record.request.chain_id,
        &agent_target,
        step,
        record.request.expected_generation,
        &record.request.expected_config_digest,
        &expected_instance,
    )?;
    Ok(AgentRequestV1 {
        api_version: "oasis7.remote-operation-agent/v1".into(),
        mode: AgentModeV1::Query,
        operation_id: record.request.operation_id.clone(),
        action_id,
        request_digest,
        operation_request_digest,
        operation_request_json,
        world_id: record.request.world_id.clone(),
        chain_id: record.request.chain_id.clone(),
        target: agent_target,
        step,
        expected_generation: record.request.expected_generation,
        expected_config_digest: record.request.expected_config_digest.clone(),
        expected_instance,
    })
}

fn action_digest(
    operation_request_digest: &str,
    operation_id: &str,
    action_id: &str,
    world_id: &str,
    chain_id: &str,
    target: &AgentTargetV1,
    step: RemoteStepV1,
    expected_generation: u64,
    expected_config_digest: &str,
    expected_instance: &Option<LocalInstance>,
) -> Result<String, String> {
    io::digest(&(
        "oasis7.remote-operation-action/v1",
        operation_request_digest,
        operation_id,
        action_id,
        world_id,
        chain_id,
        target,
        step,
        expected_generation,
        expected_config_digest,
        expected_instance,
    ))
}

pub(super) fn query_record(
    record: &Record,
    step: RemoteStepV1,
) -> Result<Option<RemoteActionReceiptV1>, String> {
    let target = remote_target(record)?;
    let request = request_for(record, step)?;
    exchange(target, &request)
}

pub(super) fn apply_record(
    record: &Record,
    step: RemoteStepV1,
) -> Result<Option<RemoteActionReceiptV1>, String> {
    let target = remote_target(record)?;
    let mut request = request_for(record, step)?;
    request.mode = AgentModeV1::Apply;
    exchange(target, &request)
}

fn remote_target(record: &Record) -> Result<&LinuxSshSystemdTargetV1, String> {
    let Some(ManagedTargetV1::LinuxSshSystemd(target)) = &record.request.managed_target else {
        return Err("managed SSH/systemd target missing".into());
    };
    validate_target(target)?;
    Ok(target)
}

fn exchange(
    target: &LinuxSshSystemdTargetV1,
    request: &AgentRequestV1,
) -> Result<Option<RemoteActionReceiptV1>, String> {
    let mut transport = SshTransport;
    exchange_with(target, request, &mut transport)
}

fn exchange_with<T: AgentTransport>(
    target: &LinuxSshSystemdTargetV1,
    request: &AgentRequestV1,
    transport: &mut T,
) -> Result<Option<RemoteActionReceiptV1>, String> {
    let mut bytes = serde_json::to_vec(request).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let reply = transport.exchange(target, &bytes)?;
    let response: AgentReplyV1 = serde_json::from_slice(&reply)
        .map_err(|error| format!("invalid bounded remote helper reply: {error}"))?;
    if response.api_version != "oasis7.remote-operation-agent-reply/v1" {
        return Err("unsupported remote helper reply schema".into());
    }
    if let Some(receipt) = &response.receipt {
        if receipt.api_version != "oasis7.remote-action-receipt/v1"
            || receipt.operation_id != request.operation_id
            || receipt.action_id != request.action_id
            || receipt.request_digest != request.request_digest
            || receipt.operation_request_digest != request.operation_request_digest
            || receipt.node_id != request.target.node_id
            || receipt.world_id != request.world_id
            || receipt.chain_id != request.chain_id
            || receipt.step != request.step
            || receipt.expected_generation != request.expected_generation
            || receipt.expected_instance != request.expected_instance
        {
            return Err("remote action receipt belongs to another request/instance".into());
        }
    }
    Ok(response.receipt)
}
