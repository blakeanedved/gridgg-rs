//! Registers the three GRID GraphQL schemas with cynic.

fn main() {
    for (name, path) in [
        ("central", "specs/central-data.graphql"),
        ("series_state", "specs/series-state.graphql"),
        ("stats", "specs/stats-feed.graphql"),
    ] {
        println!("cargo:rerun-if-changed={path}");
        cynic_codegen::register_schema(name)
            .from_sdl_file(path)
            .unwrap_or_else(|e| panic!("failed to register the {name} schema from {path}: {e}"));
    }
}
