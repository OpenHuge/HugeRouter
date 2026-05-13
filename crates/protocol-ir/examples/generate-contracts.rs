fn main() -> anyhow::Result<()> {
    protocol_ir::write_contract_artifacts(protocol_ir::workspace_root())
}
