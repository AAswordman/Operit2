/// Returns the trusted source-only parser from the same build manifest as its facade.
#[allow(non_snake_case)]
pub fn readComposeDslParserSource() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/compose_dsl_parser_source.js"))
}

/// Injects the required synchronous source provider into the parser facade.
#[allow(non_snake_case)]
pub fn buildComposeDslParserDefinition(sourceProvider: &str) -> String {
    include_str!(concat!(env!("OUT_DIR"), "/compose_dsl_parser.js"))
        .replace("__OPERIT_COMPOSE_PARSER_SOURCE__", sourceProvider)
}

/// Assembles the sole source compiler for its isolated host executor and offline tests.
#[allow(non_snake_case)]
pub fn buildComposeDslCompilerDefinition(sourceProvider: &str) -> String {
    format!(
        "{}\n{}",
        buildComposeDslParserDefinition(sourceProvider),
        include_str!(concat!(env!("OUT_DIR"), "/compose_dsl_compiler.js"))
    )
}

/// Installs page execution modules without any parser or source-lowering code.
#[allow(non_snake_case)]
pub fn buildComposeDslContextBridgeDefinition() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/compose_dsl_context.js"))
}
