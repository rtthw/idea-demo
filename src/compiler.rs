//! # Compiler

use anyhow::Result;

use crate::WORKSPACE_DIR;



pub fn run(content: &str, input_filename: &str, output_filename: &str) -> Result<()> {
    let result = std::panic::catch_unwind(|| {
        let config = rustc_interface::Config {
            opts: rustc_session::config::Options {
                crate_types: vec![rustc_session::config::CrateType::Cdylib],
                externs: rustc_session::config::Externs::new(
                    [(
                        "base".to_string(),
                        rustc_session::config::ExternEntry {
                            location: rustc_session::config::ExternLocation::ExactPaths(
                                [rustc_session::utils::CanonicalizedPath::new(
                                    format!("{WORKSPACE_DIR}/target/debug/libbase.rlib").into(),
                                )]
                                .into(),
                            ),
                            is_private_dep: false,
                            add_prelude: true,
                            nounused_dep: false,
                            force: false,
                        },
                    )]
                    .into(),
                ),
                incremental: None,
                output_types: rustc_session::config::OutputTypes::new(&[(
                    rustc_session::config::OutputType::Exe,
                    Some(rustc_session::config::OutFileName::Real(
                        output_filename.into(),
                    )),
                )]),
                cg: rustc_session::config::CodegenOptions {
                    opt_level: "3".into(),
                    panic: Some(rustc_target::spec::PanicStrategy::Abort),
                    strip: rustc_session::config::Strip::Symbols,
                    ..Default::default()
                },
                verbose: true,
                ..Default::default()
            },
            crate_cfg: Vec::new(),
            crate_check_cfg: Vec::new(),
            input: rustc_session::config::Input::Str {
                name: rustc_span::FileName::Custom(input_filename.into()),
                input: content.into(),
            },
            output_dir: Some(format!("{WORKSPACE_DIR}/target/debug").into()),
            output_file: None,
            file_loader: None,
            locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES.to_owned(),
            lint_caps: Default::default(),
            psess_created: None,
            register_lints: None,
            override_queries: None,
            registry: rustc_errors::registry::Registry::new(rustc_errors::codes::DIAGNOSTICS),
            make_codegen_backend: None,
            extra_symbols: Vec::new(),
            ice_file: None,
            hash_untracked_state: None,
            using_internal_features: &rustc_driver::USING_INTERNAL_FEATURES,
        };

        rustc_interface::run_compiler(config, |compiler| {
            let sess = &compiler.sess;
            let codegen_backend = &*compiler.codegen_backend;
            let krate = rustc_interface::passes::parse(sess);
            let linker = rustc_interface::create_and_enter_global_ctxt(&compiler, krate, |tcx| {
                rustc_interface::Linker::codegen_and_build_linker(tcx, codegen_backend)
            });
            linker.link(sess, codegen_backend);
        });
    });

    if result.is_ok() {
        std::fs::rename(
            output_filename,
            format!("{WORKSPACE_DIR}/target/debug/{output_filename}"),
        )?;
    }

    result.map_err(|_| anyhow::anyhow!("failed to compile {input_filename}"))
}
