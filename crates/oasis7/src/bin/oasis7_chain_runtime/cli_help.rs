use oasis7::chain_pos_defaults;
use oasis7::runtime::RewardAssetConfig;

use super::{
    DEFAULT_CONFIG_FILE, DEFAULT_LOCAL_TEST_PROVIDER_AGENT_ID,
    DEFAULT_LOCAL_TEST_PROVIDER_OWNER_BINDING, DEFAULT_LOCAL_TEST_PROVIDER_SESSION_MODE,
    DEFAULT_NODE_ID, DEFAULT_NODE_TICK_MS, DEFAULT_REPLICATION_NETWORK_LISTEN,
    DEFAULT_REWARD_RUNTIME_RESERVE_UNITS, DEFAULT_STATUS_BIND, DEFAULT_WORLD_ID,
};

pub fn print_help() {
    let pos_defaults = chain_pos_defaults::defaults();
    println!(
        "Usage: oasis7_chain_runtime [options]\n\n\
Starts standalone chain/node runtime with status HTTP endpoints.\n\n\
Options:\n\
  --node-id <id>                    node identifier (default: {DEFAULT_NODE_ID})\n\
  --world-id <id>                   technical runtime partition id for the unified persistent world (default: {DEFAULT_WORLD_ID})\n\
  --storage-profile <name>          dev_local|release_default|soak_forensics (default: dev_local)\n\
  --traffic-profile <name>          default|triad_low_traffic (default: default)\n\
  --status-bind <host:port>         status HTTP bind (default: {DEFAULT_STATUS_BIND})\n\
  --node-role <role>                sequencer|storage|observer (default: sequencer)\n\
  --p2p-user-mode <mode>            auto_join|private_safe|public_entry (default: auto_join)\n\
  --p2p-accept-public-entry         accept auto-detected public-entry recommendation\n\
  --p2p-reject-public-entry         force conservative fallback when auto-detect suggests public entry (default)\n\
  --p2p-detected-reachability <c>   public|hybrid|private|relay_only|validator_hidden\n\
  --p2p-clear-detected-reachability clear detected reachability hint\n\
  --p2p-detected-hole-punch <s>     unknown|viable|blocked (default: unknown)\n\
  --p2p-detected-relay-available    mark relay fallback as available (default)\n\
  --p2p-detected-relay-unavailable  mark relay fallback as unavailable\n\
  --p2p-detected-probe-stable       mark auto-detection as stable (default)\n\
  --p2p-detected-probe-unstable     mark auto-detection as unstable\n\
  --p2p-deployment-mode <mode>      public|hybrid|private|relay_only|validator_hidden (default: private)\n\
  --p2p-node-role <role>            validator_core|sentry|relay|full_storage|observer_light\n\
  --p2p-source-operator <label>     canonical operator label for peer diversity policy\n\
  --p2p-source-asn <label>          canonical ASN label for peer diversity policy\n\
  --p2p-max-ipv4-subnet-active-peers <n>\n\
                                    max active peers allowed in one IPv4 /24 before blocking\n\
  --node-tick-ms <n>                worker poll/fallback interval ms (default: {DEFAULT_NODE_TICK_MS})\n\
  --pos-slot-duration-ms <n>        PoS slot duration in milliseconds (default: {slot_duration_ms})\n\
  --pos-ticks-per-slot <n>          logical ticks per PoS slot (default: {ticks_per_slot})\n\
  --pos-proposal-tick-phase <n>     proposal trigger phase within slot tick window (default: {proposal_tick_phase})\n\
  --pos-adaptive-tick-scheduler     enable adaptive wait to next logical tick boundary\n\
  --pos-no-adaptive-tick-scheduler  disable adaptive scheduler (default)\n\
  --pos-slot-clock-genesis-unix-ms <n>\n\
                                    fixed slot clock genesis unix ms (default: auto)\n\
  --pos-max-past-slot-lag <n>       max accepted inbound stale slot lag (default: {max_past_slot_lag})\n\
  --node-validator <id:stake>       add validator stake (repeatable)\n\
  --node-validator-signer-public-key <id:public_key_hex>\n\
                                    override validator signer public key (repeatable)\n\
  --node-auto-attest-all            enable auto attesting validators\n\
  --node-no-auto-attest-all         disable auto attesting validators (default)\n\
  --node-gossip-bind <addr:port>    UDP gossip bind\n\
  --node-gossip-peer <addr:port>    UDP gossip peer (repeatable, requires --node-gossip-bind)\n\
  --replication-network-listen <multiaddr>\n\
                                    libp2p replication listen addr (repeatable, default: {DEFAULT_REPLICATION_NETWORK_LISTEN})\n\
  --replication-network-peer <multiaddr>\n\
                                    libp2p replication bootstrap peer (repeatable)\n\
  --replication-remote-writer-public-key <public_key_hex>\n\
                                    extra authorized replication fetch requester (repeatable)\n\
  --network-tier-manifest <path>    load formal network tier manifest json and bootstrap peer ref\n\
  --genesis-validator-registry <path>\n\
                                    initialize empty execution world validator registry from genesis manifest\n\
  --deployment-inventory <path>     bind immutable deployment inventory to node-emitted status\n\
  --config <path>                   config file path for node keypair (default: {DEFAULT_CONFIG_FILE})\n\
  --runtime-root <path>             override chain runtime state root directory\n\
  --execution-bridge-state <path>   override execution bridge state file path\n\
  --execution-world-dir <path>      override execution world directory\n\
  --execution-records-dir <path>    override execution records directory\n\
  --provider-bootstrap-authority <path>\n\
                                    explicit JSON Runtime authority bundle; repeat per provider-backed agent\n\
  --local-test-provider-authority <path>\n\
                                    explicit DevLocal output JSON authority bundle (opt-in setup)\n\
  --local-test-provider-wasm <path>\n\
                                    real WASM artifact for the explicit DevLocal provider setup\n\
  --local-test-provider-metadata <path>\n\
                                    canonical build-suite metadata JSON for the WASM artifact\n\
  --local-test-provider-agent-id <id>\n\
                                    live starter agent to provision (default: {DEFAULT_LOCAL_TEST_PROVIDER_AGENT_ID})\n\
  --local-test-provider-owner-binding <id>\n\
                                    stable local session owner binding (default: {DEFAULT_LOCAL_TEST_PROVIDER_OWNER_BINDING})\n\
  --local-test-provider-finality-block-hash <hash>\n\
                                    explicit local finality marker, blake3:<64 lowercase hex>\n\
  --local-test-provider-session-mode <mode>\n\
                                    hosted_public_join|loopback (default: {DEFAULT_LOCAL_TEST_PROVIDER_SESSION_MODE})\n\
  --storage-root <path>             override execution CAS/storage root\n\
  --replication-root <path>         override replication root directory\n\
  --reward-runtime-enable           enable reward runtime worker (default)\n\
  --reward-runtime-disable          disable reward runtime worker\n\
  --reward-runtime-signer-node-id <id>\n\
                                    override reward runtime signer node id (default: --node-id)\n\
  --reward-runtime-epoch-duration-secs <n>\n\
                                    override reward settlement epoch duration seconds\n\
  --reward-points-per-credit <n>    reward points per credit (default: {})\n\
  --reward-runtime-auto-redeem      enable runtime auto redeem\n\
  --reward-runtime-no-auto-redeem   disable runtime auto redeem (default)\n\
  --reward-initial-reserve-power-units <n>\n\
                                    reward reserve power units (default: {DEFAULT_REWARD_RUNTIME_RESERVE_UNITS})\n\
  --reward-distfs-probe-per-tick <n>\n\
                                    distfs challenge probes per tick (default: 1)\n\
  -h, --help                        show help",
        RewardAssetConfig::default().points_per_credit,
        slot_duration_ms = pos_defaults.slot_duration_ms,
        ticks_per_slot = pos_defaults.ticks_per_slot,
        proposal_tick_phase = pos_defaults.proposal_tick_phase,
        max_past_slot_lag = pos_defaults.max_past_slot_lag,
    );
}
