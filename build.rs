use vergen_git2::Build;
use vergen_git2::Emitter;
use vergen_git2::Git2;

pub fn main() -> anyhow::Result<()> {
    let build = Build::all_build();
    let git2 = Git2::all_git();

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&git2)?
        .emit()
}
