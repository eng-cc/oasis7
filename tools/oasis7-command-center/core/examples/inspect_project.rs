use oasis7_command_center_core::CommandCenter;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("repository path required");
    let started = std::time::Instant::now();
    let mut core = CommandCenter::open(":memory:")?;
    let snapshot = core.open_project(path)?;
    println!(
        "{}",
        serde_json::json!({"project":snapshot.project,"sources":snapshot.sources,"sections":snapshot.sections.len(),"phase":core.search("当前阶段交付目标与全局",10)?,"elapsedMs":started.elapsed().as_millis().to_string()})
    );
    Ok(())
}
