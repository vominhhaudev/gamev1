fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(&["worker.proto"], &["."])?;
    Ok(())
}
