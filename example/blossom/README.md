# The Blossom Template

This is a template project made to be as compatible as possible with [Blossom],
a small framework for creating 4K Executable Graphics artworks for the demoscene,
developed by [yx] aka [Lunasorcery].

Please checkout https://github.com/lunasorcery/Blossom for more!

[Blossom]: https://github.com/lunasorcery/Blossom
[Lunasorcery]: https://github.com/lunasorcery
[yx]: https://demozoo.org/sceners/77056/

## Pro Tip

Since the `pipeline.yaml` file does not interfere with the Blossom framework,
you can copy this folder directly into the root of the Blossom repository and
run Sh4derJockey from inside the `blossom/` subfolder.

This way you can prototype your entry with Sh4derJockey and compile the result
in Visual Studio without moving any files around.

Do keep in mind that - as of writing this - the shader minifier is not able to
optimize away any preprocessor instructions, so we recommend manually removing
the compatibility header for the final release.
