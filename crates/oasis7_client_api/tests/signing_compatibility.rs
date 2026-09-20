use oasis7_client_api::{
    ClientSigningError, MainTokenActionAuthProof, MainTokenActionAuthScheme, MainTokenTransfer,
    build_main_token_transfer_signing_payload, build_signed_main_token_transfer_request,
    sign_main_token_transfer, verify_main_token_transfer_signature,
};

const BROWSER_V1_PRIVATE_KEY: &str =
    "c7a149783d4d97d4b36f6f97ae43eb71af7fe595b7f717d329c96be3e58fdc29";
const BROWSER_V1_PUBLIC_KEY: &str =
    "fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3";
const BROWSER_V1_SIGNATURE: &str = "octransferauth:v1:9200ab505c80b5d27fb1a4e79624a083f2047669404d8c7d562e7d6b7da9154fcf7b85e96ab4c02bd5f2910845e4a7ce791d87398e0e3b9004bb022749d6830b";

const V2_PRIVATE_KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const V2_PUBLIC_KEY: &str = "d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737";
const V2_TO_ACCOUNT: &str =
    "oc:pk:204040e364c10f2bec9c1fe500a1cd4c247c89d650a01ed7e82caba867877c21";
const V2_SIGNATURE: &str = "octransferauth:v2:4c4a7f577c06ed0f1c07966b445de2b271201c779a3224fff4cd66d2ca413b1fbafa609c20023312abeba9a16cfbbc9418d03d2c810e5b3ae4aa20427cbaca0b";

fn browser_v1_transfer() -> MainTokenTransfer {
    MainTokenTransfer {
        from_account_id: format!("oc:pk:{BROWSER_V1_PUBLIC_KEY}"),
        to_account_id: "oc:pk:1111111111111111111111111111111111111111111111111111111111111111"
            .to_string(),
        amount: 1,
        nonce: 1,
        asset_id: None,
        memo: None,
        chain_id: None,
        network_id: None,
        tx_version: None,
        tx_type: None,
        valid_until_unix_ms: None,
        max_fee: None,
        fee_asset_id: None,
        application_payload_hash: None,
        client_request_id: None,
    }
}

fn v2_transfer() -> MainTokenTransfer {
    MainTokenTransfer {
        from_account_id: format!("oc:pk:{V2_PUBLIC_KEY}"),
        to_account_id: V2_TO_ACCOUNT.to_string(),
        amount: 9,
        nonce: 4,
        asset_id: Some("main_token".into()),
        memo: Some("bridge:deposit:alpha".into()),
        chain_id: Some("oasis7-main".into()),
        network_id: Some("prod".into()),
        tx_version: Some(2),
        tx_type: Some("asset_transfer".into()),
        valid_until_unix_ms: Some(1_900_000_000_000),
        max_fee: Some(42),
        fee_asset_id: Some("main_token".into()),
        application_payload_hash: Some("sha256:payload-alpha".into()),
        client_request_id: Some("client-alpha".into()),
    }
}

#[test]
fn browser_captured_v1_vector_preserves_payload_and_signature() {
    let transfer = browser_v1_transfer();
    let expected_payload = format!(
        r#"{{"version":1,"operation":"transfer_main_token","account_id":"oc:pk:{BROWSER_V1_PUBLIC_KEY}","public_key":"{BROWSER_V1_PUBLIC_KEY}","action":{{"type":"TransferMainToken","data":{{"from_account_id":"oc:pk:{BROWSER_V1_PUBLIC_KEY}","to_account_id":"oc:pk:1111111111111111111111111111111111111111111111111111111111111111","amount":1,"nonce":1}}}}}}"#,
    );
    let payload = build_main_token_transfer_signing_payload(
        &transfer,
        transfer.from_account_id.as_str(),
        BROWSER_V1_PUBLIC_KEY,
    )
    .expect("browser payload should encode");
    assert_eq!(payload, expected_payload.as_bytes());

    let proof = sign_main_token_transfer(
        &transfer,
        transfer.from_account_id.as_str(),
        BROWSER_V1_PUBLIC_KEY,
        BROWSER_V1_PRIVATE_KEY,
    )
    .expect("browser vector should sign");
    assert_eq!(proof.scheme, MainTokenActionAuthScheme::Ed25519);
    assert_eq!(proof.account_id, transfer.from_account_id);
    assert_eq!(proof.public_key.as_deref(), Some(BROWSER_V1_PUBLIC_KEY));
    assert_eq!(proof.signature.as_deref(), Some(BROWSER_V1_SIGNATURE));
    verify_main_token_transfer_signature(&transfer, &proof)
        .expect("browser-captured signature should verify");
}

#[test]
fn v2_canonical_vector_preserves_context_payload_and_signature() {
    let transfer = v2_transfer();
    let expected_payload = format!(
        r#"{{"version":1,"operation":"transfer_main_token","account_id":"oc:pk:{V2_PUBLIC_KEY}","public_key":"{V2_PUBLIC_KEY}","action":{{"type":"TransferMainToken","data":{{"from_account_id":"oc:pk:{V2_PUBLIC_KEY}","to_account_id":"{V2_TO_ACCOUNT}","amount":9,"nonce":4,"asset_id":"main_token","memo":"bridge:deposit:alpha","chain_id":"oasis7-main","network_id":"prod","tx_version":2,"tx_type":"asset_transfer","valid_until_unix_ms":1900000000000,"max_fee":42,"fee_asset_id":"main_token","application_payload_hash":"sha256:payload-alpha","client_request_id":"client-alpha"}}}}}}"#,
    );
    let payload = build_main_token_transfer_signing_payload(
        &transfer,
        transfer.from_account_id.as_str(),
        V2_PUBLIC_KEY,
    )
    .expect("v2 payload should encode");
    assert_eq!(payload, expected_payload.as_bytes());

    let proof = sign_main_token_transfer(
        &transfer,
        transfer.from_account_id.as_str(),
        V2_PUBLIC_KEY,
        V2_PRIVATE_KEY,
    )
    .expect("v2 vector should sign");
    assert_eq!(proof.signature.as_deref(), Some(V2_SIGNATURE));
    verify_main_token_transfer_signature(&transfer, &proof)
        .expect("v2 canonical signature should verify");

    let request = build_signed_main_token_transfer_request(&transfer, &proof)
        .expect("v2 wire request should build");
    assert_eq!(request.transfer, transfer);
    assert_eq!(request.public_key, V2_PUBLIC_KEY);
    assert_eq!(request.signature, V2_SIGNATURE);
}

#[test]
fn altered_v2_context_is_rejected_without_resigning() {
    let transfer = v2_transfer();
    let proof = sign_main_token_transfer(
        &transfer,
        transfer.from_account_id.as_str(),
        V2_PUBLIC_KEY,
        V2_PRIVATE_KEY,
    )
    .expect("v2 vector should sign");
    let mut altered = transfer.clone();
    altered.network_id = Some("staging".into());
    let error = verify_main_token_transfer_signature(&altered, &proof)
        .expect_err("altered network context must not verify");
    assert!(matches!(error, ClientSigningError::InvalidSignature(_)));
}

#[test]
fn mismatched_signing_key_and_public_key_are_rejected() {
    let transfer = v2_transfer();
    let error = sign_main_token_transfer(
        &transfer,
        transfer.from_account_id.as_str(),
        V2_PUBLIC_KEY,
        BROWSER_V1_PRIVATE_KEY,
    )
    .expect_err("private key from another signer must be rejected");
    assert!(matches!(error, ClientSigningError::InvalidRequest(_)));

    let mut proof = sign_main_token_transfer(
        &transfer,
        transfer.from_account_id.as_str(),
        V2_PUBLIC_KEY,
        V2_PRIVATE_KEY,
    )
    .expect("v2 vector should sign");
    proof.public_key = Some(BROWSER_V1_PUBLIC_KEY.to_string());
    let error = verify_main_token_transfer_signature(&transfer, &proof)
        .expect_err("wrong signer key must be rejected");
    assert!(matches!(error, ClientSigningError::AccountMismatch(_)));
}

#[test]
fn signed_request_rejects_threshold_proof() {
    let transfer = browser_v1_transfer();
    let proof = MainTokenActionAuthProof {
        scheme: MainTokenActionAuthScheme::ThresholdEd25519,
        account_id: transfer.from_account_id.clone(),
        public_key: None,
        signature: None,
        threshold: Some(1),
        participant_signatures: Vec::new(),
    };
    let error = build_signed_main_token_transfer_request(&transfer, &proof)
        .expect_err("wire helper must reject threshold proof");
    assert!(matches!(error, ClientSigningError::InvalidRequest(_)));
}
