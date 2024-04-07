use vergen::EmitBuilder;

pub fn main() -> anyhow::Result<()> {
    EmitBuilder::builder()
        .all_build()
        //.all_cargo()
        .all_git()
        //.all_rustc()
        //.all_sysinfo()
        .emit()
}
