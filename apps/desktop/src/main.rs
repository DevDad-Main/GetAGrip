//! GetAGrip — desktop entry point.

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("GetAGrip v{} starting...", atlas_core::VERSION);

    let config = atlas_core::WorkspaceConfig::default();
    tracing::info!(
        "Loaded config: theme={}, font={}, tab_width={}",
        config.appearance.theme,
        config.editor.font_family,
        config.editor.tab_width,
    );

    let profiles = atlas_core::session::ConnectionProfiles::new();
    tracing::info!("Connection profiles loaded: {} profiles", profiles.profiles.len());

    tracing::info!("GetAGrip is ready (Phase 1 — core initialisation).");
}
