fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .compile(&["../proto/tempo/v1/tempo.proto"], &["../proto"])?;
    println!("cargo:rerun-if-changed=../proto/tempo/v1/tempo.proto");
    Ok(())
}
