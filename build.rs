use vergen::{vergen, Config};

fn main() -> Result<(), anyhow::Error> {
    // Generate the default 'cargo:' instruction output
    vergen(Config::default())
}
